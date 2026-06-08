use std::collections::HashMap;

use tower_lsp::lsp_types::{Location, Position, Range, SymbolInformation, SymbolKind, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolLanguage {
    Renpy,
    Python,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexedSymbolKind {
    Label,
    Screen,
    Transform,
    Style,
    Image,
    Variable,
    Function,
    Class,
    Import,
    Assignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexedReferenceKind {
    Label,
    Screen,
    Transform,
    Style,
    Image,
    Variable,
    Python,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationRange {
    pub uri: Url,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: IndexedSymbolKind,
    pub language: SymbolLanguage,
    pub definition: LocationRange,
    pub detail: Option<String>,
    pub fragment_id: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct IndexedReference {
    pub name: String,
    pub kind: IndexedReferenceKind,
    pub language: SymbolLanguage,
    pub range: LocationRange,
    pub resolved_to: Vec<SymbolId>,
    pub fragment_id: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    pub symbols: Vec<IndexedSymbol>,
    pub references: Vec<IndexedReference>,
    names: HashMap<(IndexedSymbolKind, String), Vec<SymbolId>>,
}

impl SymbolIndex {
    pub fn push_symbol(
        &mut self,
        name: impl Into<String>,
        kind: IndexedSymbolKind,
        language: SymbolLanguage,
        definition: LocationRange,
        detail: Option<String>,
        fragment_id: Option<usize>,
    ) -> SymbolId {
        let name = name.into();
        let id = SymbolId(self.symbols.len());
        self.symbols.push(IndexedSymbol {
            id,
            name: name.clone(),
            kind,
            language,
            definition,
            detail,
            fragment_id,
        });
        self.names.entry((kind, name)).or_default().push(id);
        id
    }

    pub fn push_reference(
        &mut self,
        name: impl Into<String>,
        kind: IndexedReferenceKind,
        language: SymbolLanguage,
        range: LocationRange,
        fragment_id: Option<usize>,
    ) {
        let name = name.into();
        let resolved_to = self
            .lookup_reference_targets(kind, &name)
            .into_iter()
            .collect::<Vec<_>>();
        self.references.push(IndexedReference {
            name,
            kind,
            language,
            range,
            resolved_to,
            fragment_id,
        });
    }

    pub fn lookup_reference_targets(&self, kind: IndexedReferenceKind, name: &str) -> Vec<SymbolId> {
        match kind {
            IndexedReferenceKind::Label => self.lookup_symbols_by_kind(IndexedSymbolKind::Label, name),
            IndexedReferenceKind::Screen => {
                self.lookup_symbols_by_kind(IndexedSymbolKind::Screen, name)
            }
            IndexedReferenceKind::Transform => {
                self.lookup_symbols_by_kind(IndexedSymbolKind::Transform, name)
            }
            IndexedReferenceKind::Style => self.lookup_symbols_by_kind(IndexedSymbolKind::Style, name),
            IndexedReferenceKind::Image => self.lookup_symbols_by_kind(IndexedSymbolKind::Image, name),
            IndexedReferenceKind::Variable => {
                self.lookup_symbols_by_kind(IndexedSymbolKind::Variable, name)
            }
            IndexedReferenceKind::Python => {
                let mut targets = self.lookup_symbols_by_kind(IndexedSymbolKind::Function, name);
                targets.extend(self.lookup_symbols_by_kind(IndexedSymbolKind::Class, name));
                targets.extend(self.lookup_symbols_by_kind(IndexedSymbolKind::Assignment, name));
                targets.extend(self.lookup_symbols_by_kind(IndexedSymbolKind::Import, name));
                targets
            }
        }
    }

    pub fn symbol(&self, id: SymbolId) -> Option<&IndexedSymbol> {
        self.symbols.get(id.0)
    }

    pub fn symbol_at(&self, uri: &Url, position: Position) -> Option<&IndexedSymbol> {
        self.symbols.iter().find(|symbol| {
            symbol.definition.uri == *uri && range_contains(&symbol.definition.range, position)
        })
    }

    pub fn reference_at(&self, uri: &Url, position: Position) -> Option<&IndexedReference> {
        self.references
            .iter()
            .find(|reference| reference.range.uri == *uri && range_contains(&reference.range.range, position))
    }

    pub fn goto_definition(&self, uri: &Url, position: Position) -> Option<Location> {
        if let Some(symbol) = self.symbol_at(uri, position) {
            if let Some(previous_definition) = self.preferred_definition_for_symbol(symbol) {
                return Some(Location::new(
                    previous_definition.definition.uri.clone(),
                    previous_definition.definition.range,
                ));
            }
            return Some(Location::new(
                symbol.definition.uri.clone(),
                symbol.definition.range,
            ));
        }

        let reference = self.reference_at(uri, position)?;
        let target = reference
            .resolved_to
            .iter()
            .find_map(|id| self.symbol(*id))
            .or_else(|| {
                self.symbols.iter().find(|symbol| {
                    symbol.name == reference.name
                        && symbol_kind_matches_reference(symbol.kind, reference.kind)
                })
            })?;
        Some(Location::new(
            target.definition.uri.clone(),
            target.definition.range,
        ))
    }

    pub fn hover(&self, uri: &Url, position: Position) -> Option<String> {
        if let Some(symbol) = self.symbol_at(uri, position) {
            return Some(symbol_hover(symbol));
        }
        let reference = self.reference_at(uri, position)?;
        let target = reference
            .resolved_to
            .iter()
            .find_map(|id| self.symbol(*id))
            .or_else(|| {
                self.symbols.iter().find(|symbol| {
                    symbol.name == reference.name
                        && symbol_kind_matches_reference(symbol.kind, reference.kind)
                })
            })?;
        Some(symbol_hover(target))
    }

    #[allow(deprecated)]
    pub fn document_symbols(&self, uri: &Url) -> Vec<SymbolInformation> {
        self.symbols
            .iter()
            .filter(|symbol| symbol.definition.uri == *uri)
            .map(|symbol| SymbolInformation {
                name: symbol.name.clone(),
                kind: to_lsp_symbol_kind(symbol.kind),
                tags: None,
                deprecated: None,
                location: Location::new(symbol.definition.uri.clone(), symbol.definition.range),
                container_name: symbol.detail.clone(),
            })
            .collect()
    }

    #[allow(deprecated)]
    pub fn workspace_symbols(&self, query: &str) -> Vec<SymbolInformation> {
        let query = query.to_ascii_lowercase();
        self.symbols
            .iter()
            .filter(|symbol| query.is_empty() || symbol.name.to_ascii_lowercase().contains(&query))
            .map(|symbol| SymbolInformation {
                name: symbol.name.clone(),
                kind: to_lsp_symbol_kind(symbol.kind),
                tags: None,
                deprecated: None,
                container_name: symbol.detail.clone(),
                location: Location::new(
                    symbol.definition.uri.clone(),
                    symbol.definition.range,
                ),
            })
            .collect()
    }
}

impl SymbolIndex {
    fn lookup_symbols_by_kind(&self, kind: IndexedSymbolKind, name: &str) -> Vec<SymbolId> {
        self.names
            .get(&(kind, name.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    fn preferred_definition_for_symbol(&self, symbol: &IndexedSymbol) -> Option<&IndexedSymbol> {
        if !matches!(symbol.kind, IndexedSymbolKind::Assignment | IndexedSymbolKind::Variable) {
            return None;
        }

        self.symbols
            .iter()
            .filter(|candidate| {
                candidate.id != symbol.id
                    && candidate.name == symbol.name
                    && candidate.kind == symbol.kind
                    && candidate.definition.uri == symbol.definition.uri
                    && candidate.definition.range.start.line <= symbol.definition.range.start.line
            })
            .min_by_key(|candidate| {
                (
                    candidate.definition.range.start.line,
                    candidate.definition.range.start.character,
                )
            })
    }
}

fn symbol_kind_matches_reference(symbol: IndexedSymbolKind, reference: IndexedReferenceKind) -> bool {
    matches!(
        (symbol, reference),
        (IndexedSymbolKind::Label, IndexedReferenceKind::Label)
            | (IndexedSymbolKind::Screen, IndexedReferenceKind::Screen)
            | (IndexedSymbolKind::Transform, IndexedReferenceKind::Transform)
            | (IndexedSymbolKind::Style, IndexedReferenceKind::Style)
            | (IndexedSymbolKind::Image, IndexedReferenceKind::Image)
            | (IndexedSymbolKind::Variable, IndexedReferenceKind::Variable)
            | (IndexedSymbolKind::Assignment, IndexedReferenceKind::Python)
            | (IndexedSymbolKind::Function, IndexedReferenceKind::Python)
            | (IndexedSymbolKind::Class, IndexedReferenceKind::Python)
            | (IndexedSymbolKind::Import, IndexedReferenceKind::Python)
    )
}

fn symbol_hover(symbol: &IndexedSymbol) -> String {
    match &symbol.detail {
        Some(detail) => format!("{} `{}`", detail, symbol.name),
        None => format!("{:?} `{}`", symbol.kind, symbol.name),
    }
}

fn to_lsp_symbol_kind(kind: IndexedSymbolKind) -> SymbolKind {
    match kind {
        IndexedSymbolKind::Label => SymbolKind::FUNCTION,
        IndexedSymbolKind::Screen => SymbolKind::OBJECT,
        IndexedSymbolKind::Transform => SymbolKind::FUNCTION,
        IndexedSymbolKind::Style => SymbolKind::CLASS,
        IndexedSymbolKind::Image => SymbolKind::OBJECT,
        IndexedSymbolKind::Variable => SymbolKind::VARIABLE,
        IndexedSymbolKind::Function => SymbolKind::FUNCTION,
        IndexedSymbolKind::Class => SymbolKind::CLASS,
        IndexedSymbolKind::Import => SymbolKind::MODULE,
        IndexedSymbolKind::Assignment => SymbolKind::VARIABLE,
    }
}

fn range_contains(range: &Range, position: Position) -> bool {
    (position.line > range.start.line
        || (position.line == range.start.line && position.character >= range.start.character))
        && (position.line < range.end.line
            || (position.line == range.end.line && position.character <= range.end.character))
}
