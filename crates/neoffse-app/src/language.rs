//! Game-data language of the running desktop session.

use neoffse::data::Language;

/// Game-data language for a locale tag: `Language::Fr` when the primary
/// subtag is "fr", otherwise `Language::En`.
///
/// The primary subtag is the text before the first `-`, `_`, `.` or `@`.
/// The comparison ignores ASCII case, so `fr`, `FR`, `fr-FR`, `fr_CA.UTF-8`
/// and `fr@euro` give French, while `fra`, `fr2`, `C` and `None` give English.
#[must_use]
pub fn language_for_locale(locale: Option<&str>) -> Language {
    let primary = locale.and_then(|tag| tag.split(['-', '_', '.', '@']).next());
    if primary.is_some_and(|subtag| subtag.eq_ignore_ascii_case("fr")) {
        Language::Fr
    } else {
        Language::En
    }
}

/// Reads the system locale (`sys_locale::get_locale`) and applies
/// [`language_for_locale`].
#[must_use]
pub fn system_language() -> Language {
    language_for_locale(sys_locale::get_locale().as_deref())
}
