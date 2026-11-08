import Foundation

/// Italiano o inglese, come la lingua del telefono (le altre lingue della
/// desktop arriveranno). `L("italiano", "english")`.
let italian: Bool = Locale.preferredLanguages.first?.hasPrefix("it") ?? false

func L(_ it: String, _ en: String) -> String { italian ? it : en }

/// Con argomenti: `Lf("Pagina %d di %d", "Page %d of %d", 1, 3)`.
func Lf(_ it: String, _ en: String, _ args: CVarArg...) -> String {
    String(format: italian ? it : en, arguments: args)
}

/// Il nome di un foglio (codice di `src/rigatura.rs`).
func paperName(_ code: String) -> String {
    let names: [String: (String, String)] = [
        "bianco": ("Bianco", "Blank"),
        "righe": ("Righe (8 mm)", "Lined (8 mm)"),
        "righe10": ("Righe larghe (1 cm)", "Wide lined (1 cm)"),
        "elem1": ("Righe di 1ª elementare", "Year 1 school lines"),
        "elem2": ("Righe di 2ª elementare", "Year 2 school lines"),
        "elem3": ("Righe di 3ª elementare", "Year 3 school lines"),
        "quadretti5": ("Quadretti piccoli (5 mm)", "Small squares (5 mm)"),
        "quadretti5m": ("Quadretti 5 mm con margine", "5 mm squares with margin"),
        "quadretti4": ("Quadretti piccoli (4 mm)", "Small squares (4 mm)"),
        "quadretti10": ("Quadretti grandi (1 cm)", "Large squares (1 cm)"),
        "millimetrata": ("Carta millimetrata", "Graph paper"),
        "cartesiano": ("Piano cartesiano", "Cartesian plane"),
        "puntinato": ("Puntinato (5 mm)", "Dotted (5 mm)"),
        "isometrica": ("Isometrica (triangoli)", "Isometric (triangles)"),
        "pentagramma": ("Pentagramma", "Music staff"),
        "cornell": ("Appunti Cornell", "Cornell notes"),
    ]
    guard let n = names[code] else { return code }
    return L(n.0, n.1)
}

/// I gruppi del menu dei fogli (chiavi della desktop).
func paperGroupName(_ key: String) -> String {
    switch key {
    case "Righe": return L("Righe", "Lined")
    case "Quadretti": return L("Quadretti", "Squared")
    case "Matematica e disegno": return L("Matematica e disegno", "Maths and drawing")
    case "Altro": return L("Altro", "Other")
    default: return key
    }
}

/// Le scritte dei fogli (Cornell).
func paperLabel(_ key: String) -> String {
    switch key {
    case "Parole chiave": return L("Parole chiave", "Keywords")
    case "Appunti": return L("Appunti", "Notes")
    case "Riassunto": return L("Riassunto", "Summary")
    default: return key
    }
}
