use std::collections::HashMap;
use std::sync::Arc;

use tower_lsp::lsp_types::{Location, Position, SymbolInformation, Url};

use crate::index::SymbolIndex;
use crate::parse::ParsedDocument;

#[derive(Debug, Default)]
pub struct WorkspaceState {
    documents: HashMap<Url, Arc<ParsedDocument>>,
}

impl WorkspaceState {
    pub fn upsert(&mut self, document: ParsedDocument) -> Arc<ParsedDocument> {
        let uri = document
            .source
            .file_url()
            .expect("workspace document should have a file url");
        let document = Arc::new(document);
        self.documents.insert(uri, document.clone());
        document
    }

    pub fn remove(&mut self, uri: &Url) {
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

    pub fn goto_definition(&self, uri: &Url, position: Position) -> Option<Location> {
        self.documents.values().find_map(|document| {
            document
                .symbols
                .goto_definition(uri, position)
                .or_else(|| lookup_in_related_documents(&document.symbols, uri, position))
        })
    }
}

fn lookup_in_related_documents(
    _symbols: &SymbolIndex,
    _uri: &Url,
    _position: Position,
) -> Option<Location> {
    None
}
