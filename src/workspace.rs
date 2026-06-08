use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tower_lsp::lsp_types::{Location, Position, SymbolInformation, Url};
use walkdir::WalkDir;

use crate::index::{IndexedReferenceKind, IndexedSymbol, IndexedSymbolKind};
use crate::parse::{ParseOptions, ParsedDocument, parse_document};
use crate::source::SourceDocument;

#[derive(Debug, Default)]
pub struct WorkspaceState {
    documents: HashMap<Url, Arc<ParsedDocument>>,
    open_documents: HashSet<Url>,
    roots: Vec<PathBuf>,
}

impl WorkspaceState {
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        let mut roots = roots;
        roots.sort();
        roots.dedup();
        self.roots = roots;
    }

    pub fn index_workspace(&mut self) {
        let files = self
            .roots
            .iter()
            .flat_map(|root| collect_workspace_files(root))
            .collect::<Vec<_>>();
        let mut seen_workspace_uris = HashSet::new();

        for path in files {
            let Some(uri) = Url::from_file_path(&path).ok() else {
                continue;
            };
            seen_workspace_uris.insert(uri.clone());
            if self.open_documents.contains(&uri) {
                continue;
            }
            let Some(document) = load_document_from_disk(path, Some(uri.clone())) else {
                continue;
            };
            self.documents.insert(uri, Arc::new(document));
        }

        self.documents.retain(|uri, _| {
            self.open_documents.contains(uri) || seen_workspace_uris.contains(uri)
        });
    }

    pub fn upsert(&mut self, document: ParsedDocument, is_open: bool) -> Arc<ParsedDocument> {
        let uri = document
            .source
            .file_url()
            .expect("workspace document should have a file url");
        let document = Arc::new(document);
        if is_open {
            self.open_documents.insert(uri.clone());
        }
        self.documents.insert(uri, document.clone());
        document
    }

    pub fn close(&mut self, uri: &Url) {
        self.open_documents.remove(uri);
        let Some(path) = uri.to_file_path().ok() else {
            self.documents.remove(uri);
            return;
        };

        if self.is_in_workspace(&path)
            && path.is_file()
            && let Some(document) = load_document_from_disk(path, Some(uri.clone()))
        {
            self.documents.insert(uri.clone(), Arc::new(document));
            return;
        }

        self.documents.remove(uri);
    }

    pub fn document(&self, uri: &Url) -> Option<Arc<ParsedDocument>> {
        self.documents.get(uri).cloned()
    }

    pub fn workspace_symbols(&self, query: &str) -> Vec<SymbolInformation> {
        self.documents
            .values()
            .flat_map(|document| document.symbols.workspace_symbols(query))
            .collect()
    }

    pub fn goto_definition(&self, uri: &Url, position: Position) -> Option<Vec<Location>> {
        let document = self.documents.get(uri)?;
        let reference = document.symbols.reference_at(uri, position);
        let symbol = document.symbols.symbol_at(uri, position);

        if let Some(reference) = reference {
            let matches = self.matching_symbols_for_reference(reference.kind, &reference.name);
            if !matches.is_empty() {
                let locations = matches
                    .into_iter()
                    .map(symbol_to_location)
                    .collect::<Vec<_>>();
                return Some(dedup_locations(locations));
            }
        }

        if let Some(symbol) = symbol {
            if let Some(location) = document.symbols.goto_definition(uri, position) {
                if self.should_expand_symbol_definition(symbol) {
                    let mut locations = vec![location];
                    locations.extend(
                        self.matching_symbols_for_symbol(symbol)
                            .into_iter()
                            .map(symbol_to_location),
                    );
                    return Some(dedup_locations(locations));
                }

                return Some(vec![location]);
            }
        }

        None
    }

    pub fn hover(&self, uri: &Url, position: Position) -> Option<String> {
        let document = self.documents.get(uri)?;
        if let Some(symbol) = document.symbols.symbol_at(uri, position) {
            return Some(symbol_hover_contents(symbol));
        }
        let reference = document.symbols.reference_at(uri, position)?;
        let target = self
            .matching_symbols_for_reference(reference.kind, &reference.name)
            .into_iter()
            .next()?;
        Some(symbol_hover_contents(target))
    }

    fn matching_symbols_for_reference(
        &self,
        kind: IndexedReferenceKind,
        name: &str,
    ) -> Vec<&IndexedSymbol> {
        self.documents
            .values()
            .flat_map(|document| document.symbols.symbols.iter())
            .filter(|symbol| {
                symbol.name == name
                    && matches_reference_kind(symbol.kind, kind)
            })
            .collect()
    }

    fn matching_symbols_for_symbol(&self, symbol: &IndexedSymbol) -> Vec<&IndexedSymbol> {
        let mut matches = self
            .documents
            .values()
            .flat_map(|document| document.symbols.symbols.iter())
            .filter(|candidate| {
                candidate.name == symbol.name
                    && candidate.kind == symbol.kind
                    && !same_location(&candidate.definition, &symbol.definition)
            })
            .collect::<Vec<_>>();

        matches.sort_by_key(|candidate| {
            (
                candidate.definition.uri.to_string(),
                candidate.definition.range.start.line,
                candidate.definition.range.start.character,
            )
        });
        matches
    }

    fn is_in_workspace(&self, path: &Path) -> bool {
        self.roots.iter().any(|root| path.starts_with(root))
    }

    fn should_expand_symbol_definition(&self, symbol: &IndexedSymbol) -> bool {
        matches!(
            symbol.kind,
            IndexedSymbolKind::Assignment
                | IndexedSymbolKind::Variable
                | IndexedSymbolKind::Function
                | IndexedSymbolKind::Class
                | IndexedSymbolKind::Import
        )
    }
}

fn load_document_from_disk(path: PathBuf, uri: Option<Url>) -> Option<ParsedDocument> {
    let text = fs::read_to_string(&path).ok()?;
    Some(parse_document(
        SourceDocument::new(uri, path, text),
        ParseOptions,
    ))
}

fn collect_workspace_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if matches!(extension, "rpy" | "py") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

fn symbol_to_location(symbol: &IndexedSymbol) -> Location {
    Location::new(symbol.definition.uri.clone(), symbol.definition.range)
}

fn symbol_hover_contents(symbol: &IndexedSymbol) -> String {
    match &symbol.detail {
        Some(detail) => format!("{} `{}`", detail, symbol.name),
        None => format!("{:?} `{}`", symbol.kind, symbol.name),
    }
}

fn dedup_locations(locations: Vec<Location>) -> Vec<Location> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for location in locations {
        let key = (
            location.uri.to_string(),
            location.range.start.line,
            location.range.start.character,
            location.range.end.line,
            location.range.end.character,
        );
        if seen.insert(key) {
            deduped.push(location);
        }
    }
    deduped
}

fn same_location(left: &crate::index::LocationRange, right: &crate::index::LocationRange) -> bool {
    left.uri == right.uri && left.range == right.range
}

fn matches_reference_kind(symbol_kind: IndexedSymbolKind, reference_kind: IndexedReferenceKind) -> bool {
    matches!(
        (symbol_kind, reference_kind),
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use tower_lsp::lsp_types::Position;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("renpyfmt-workspace-{name}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn indexes_workspace_files_from_disk() {
        let root = temp_dir("index");
        let a = root.join("a.rpy");
        let b = root.join("nested").join("b.rpy");
        fs::create_dir_all(b.parent().unwrap()).unwrap();
        fs::write(&a, "label start:\n    jump elsewhere\n").unwrap();
        fs::write(&b, "label elsewhere:\n    pass\n").unwrap();

        let mut workspace = WorkspaceState::default();
        workspace.set_roots(vec![root.clone()]);
        workspace.index_workspace();

        assert!(workspace
            .document(&Url::from_file_path(&a).unwrap())
            .is_some());
        assert!(workspace
            .document(&Url::from_file_path(&b).unwrap())
            .is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_cross_file_label_definition() {
        let root = temp_dir("cross-file");
        let a = root.join("a.rpy");
        let b = root.join("b.rpy");
        fs::write(&a, "label start:\n    jump elsewhere\n").unwrap();
        fs::write(&b, "label elsewhere:\n    pass\n").unwrap();

        let mut workspace = WorkspaceState::default();
        workspace.set_roots(vec![root.clone()]);
        workspace.index_workspace();

        let uri = Url::from_file_path(&a).unwrap();
        let locations = workspace
            .goto_definition(&uri, Position::new(1, 10))
            .expect("cross-file definition");

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, Url::from_file_path(&b).unwrap());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_cross_file_python_definition_from_embedded_python() {
        let root = temp_dir("cross-file-python");
        let a = root.join("defs.rpy");
        let b = root.join("use.rpy");
        fs::write(
            &a,
            concat!(
                "init python:\n",
                "    def day_planner():\n",
                "        return 1\n",
            ),
        )
        .unwrap();
        fs::write(
            &b,
            concat!(
                "label start:\n",
                "    $ day_planner()\n",
            ),
        )
        .unwrap();

        let mut workspace = WorkspaceState::default();
        workspace.set_roots(vec![root.clone()]);
        workspace.index_workspace();

        let uri = Url::from_file_path(&b).unwrap();
        let locations = workspace
            .goto_definition(&uri, Position::new(1, 6))
            .expect("cross-file python definition");

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, Url::from_file_path(&a).unwrap());
        assert_eq!(locations[0].range.start.line, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hover_uses_workspace_symbols_for_cross_file_python_reference() {
        let root = temp_dir("cross-file-hover");
        let a = root.join("defs.rpy");
        let b = root.join("use.rpy");
        fs::write(
            &a,
            concat!(
                "init python:\n",
                "    def day_planner():\n",
                "        return 1\n",
            ),
        )
        .unwrap();
        fs::write(
            &b,
            concat!(
                "label start:\n",
                "    $ day_planner()\n",
            ),
        )
        .unwrap();

        let mut workspace = WorkspaceState::default();
        workspace.set_roots(vec![root.clone()]);
        workspace.index_workspace();

        let hover = workspace
            .hover(&Url::from_file_path(&b).unwrap(), Position::new(1, 6))
            .expect("cross-file hover");

        assert_eq!(hover, "python function `day_planner`");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reindex_removes_deleted_workspace_files() {
        let root = temp_dir("remove-stale");
        let path = root.join("stale.rpy");
        fs::write(&path, "label stale:\n    pass\n").unwrap();

        let uri = Url::from_file_path(&path).unwrap();
        let mut workspace = WorkspaceState::default();
        workspace.set_roots(vec![root.clone()]);
        workspace.index_workspace();
        assert!(workspace.document(&uri).is_some());

        fs::remove_file(&path).unwrap();
        workspace.index_workspace();
        assert!(workspace.document(&uri).is_none());

        let _ = fs::remove_dir_all(root);
    }
}
