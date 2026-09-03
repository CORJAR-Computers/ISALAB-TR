//! # Parser CSV mínimo para importación de resultados de analizadores
//!
//! Los analizadores de mesa (MINDRAY, IDEXX, …) exportan resultados por USB en
//! CSV con comas o punto y coma y decimales en coma o punto, según el locale.
//! Este módulo implementa un parser tolerante a campos entrecomillados y a
//! saltos de línea dentro de comillas, sin dependencias externas.

/// Parsea el contenido CSV completo. Cada fila es un vector de campos; los
/// campos entrecomillados pueden contener el delimitador y saltos de línea.
pub fn parse_csv(content: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        // Comilla escapada ("") dentro del campo.
                        field.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                }
                _ => field.push(c),
            }
        } else {
            match c {
                '"' => in_quotes = true,
                c if c == delimiter => {
                    row.push(std::mem::take(&mut field));
                }
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                '\r' => {
                    // Retorno de carro: se ignora (CRLF y CR clásicos).
                }
                _ => field.push(c),
            }
        }
    }

    // Último campo/fila sin salto de línea final.
    if !field.is_empty() || !row.is_empty() {
        row.push(std::mem::take(&mut field));
        rows.push(row);
    }

    rows
}

/// Detecta el delimitador más probable (`,` o `;`) comparando la consistencia
/// del número de columnas en las primeras filas.
pub fn detect_delimiter(content: &str) -> char {
    let head: String = content.lines().take(5).collect::<Vec<_>>().join("\n");
    let score = |d: char| -> usize {
        let rows = parse_csv(&head, d);
        if rows.is_empty() {
            return 0;
        }
        let cols = rows[0].len();
        if cols <= 1 {
            return 0;
        }
        rows.iter().filter(|r| r.len() == cols).count()
    };
    if score(';') >= score(',') && score(';') > 0 {
        ';'
    } else {
        ','
    }
}

/// Convierte un campo numérico tolerando decimales con coma (locale hispano):
/// `"12,5"` → 12.5. Devuelve None si no es un número válido.
pub fn parse_number(field: &str) -> Option<f64> {
    let t = field.trim();
    if t.is_empty() {
        return None;
    }
    let normalized = if t.contains(',') && !t.contains('.') {
        t.replace(',', ".")
    } else {
        t.to_string()
    };
    normalized.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comma() {
        let rows = parse_csv("a,b,c\n1,2,3\n", ',');
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b", "c"]);
        assert_eq!(rows[1], vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_semicolon() {
        let rows = parse_csv("Codigo;Glucosa;ALT\nM-1;95,5;40\n", ';');
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], vec!["M-1", "95,5", "40"]);
    }

    #[test]
    fn test_parse_quoted_with_delimiter() {
        let rows = parse_csv("a;\"b;c\";d\n", ';');
        assert_eq!(rows[0], vec!["a", "b;c", "d"]);
    }

    #[test]
    fn test_parse_escaped_quotes() {
        let rows = parse_csv("a;\"say \"\"hi\"\"\";c\n", ';');
        assert_eq!(rows[0][1], "say \"hi\"");
    }

    #[test]
    fn test_parse_multiline_quoted() {
        let rows = parse_csv("a;\"line1\nline2\";c\n", ';');
        assert_eq!(rows[0][1], "line1\nline2");
    }

    #[test]
    fn test_detect_delimiter_prefers_semicolon() {
        let content = "Codigo;Glucosa;ALT\nM-1;95;40\nM-2;110;35\n";
        assert_eq!(detect_delimiter(content), ';');
    }

    #[test]
    fn test_detect_delimiter_comma() {
        let content = "Code,Glucose,ALT\nM-1,95,40\nM-2,110,35\n";
        assert_eq!(detect_delimiter(content), ',');
    }

    #[test]
    fn test_parse_number_decimal_comma() {
        assert_eq!(parse_number("95,5"), Some(95.5));
        assert_eq!(parse_number("40"), Some(40.0));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number(" 12.5 "), Some(12.5));
    }
}
