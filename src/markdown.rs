use pulldown_cmark::{html, Parser};

/// Convertit du texte Markdown en HTML, pour l'affichage des descriptions
/// d'éléments.
pub fn to_html(source: &str) -> String {
    let parser = Parser::new(source);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}