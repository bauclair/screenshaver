use rusqlite::Connection;


const ENGLISH_TEST_KEYS: &[&str] = &[
    "app.name",
    "target.screensaver",
    "target.wallpaper",
];


pub fn run(
    connection: &Connection,
    configured_locale: &str,
) -> Result<(), String> {

    let localization =
        crate::manage_localization::Localization::load(
            connection,
            configured_locale,
        )?;

    println!(
        "[LOCALIZATION TEST] Configured locale: {}",
        localization.requested_locale()
    );

    println!(
        "[LOCALIZATION TEST] Active locale: {}",
        localization.active_locale()
    );

    println!(
        "[LOCALIZATION TEST] Text direction: {}",
        localization.text_direction()
    );

    println!(
        "[LOCALIZATION TEST] Locale fallback used: {}",
        localization.used_fallback_locale()
    );


    for key in ENGLISH_TEST_KEYS {

        let text =
            localization.text(
                connection,
                key,
            )?;

        println!(
            "[LOCALIZATION TEST] {} = {}",
            key,
            text,
        );
    }


    // Prove locale-level fallback independently of the user's configured
    // locale. A deliberately unavailable locale must fall back to en-US.
    let fallback =
        crate::manage_localization::Localization::load(
            connection,
            "zz-ZZ",
        )?;

    if fallback.active_locale()
        != crate::manage_localization::DEFAULT_LOCALE
        || !fallback.used_fallback_locale()
    {
        return Err(
            "Unavailable-locale fallback did not select en-US"
                .to_string()
        );
    }


    let fallback_text =
        fallback.text(
            connection,
            "target.screensaver",
        )?;

    if fallback_text
        != "Screensaver"
    {
        return Err(
            format!(
                "English fallback returned unexpected text for target.screensaver: '{}'",
                fallback_text,
            )
        );
    }


    println!(
        "[LOCALIZATION TEST] Verified: unavailable locale falls back to en-US"
    );

    println!(
        "[LOCALIZATION TEST] Verified: canonical English fallback resolves from translation_keys"
    );


    // Prove a real enabled non-English locale and both lookup branches:
    // target.screensaver has an es-US translation, while target.wallpaper
    // deliberately does not and must fall back per-key to canonical English.
    let spanish =
        crate::manage_localization::Localization::load(
            connection,
            "es-US",
        )?;

    if spanish.active_locale()
        != "es-US"
        || spanish.used_fallback_locale()
    {
        return Err(
            "Enabled es-US locale was not selected directly"
                .to_string()
        );
    }


    if spanish.text_direction()
        != "ltr"
    {
        return Err(
            format!(
                "Expected es-US text direction 'ltr', found '{}'",
                spanish.text_direction(),
            )
        );
    }


    let translated_screensaver =
        spanish.text(
            connection,
            "target.screensaver",
        )?;

    if translated_screensaver
        != "Protector de pantalla"
    {
        return Err(
            format!(
                "Expected es-US translation for target.screensaver, found '{}'",
                translated_screensaver,
            )
        );
    }


    let untranslated_wallpaper =
        spanish.text(
            connection,
            "target.wallpaper",
        )?;

    if untranslated_wallpaper
        != "Wallpaper"
    {
        return Err(
            format!(
                "Expected per-key English fallback for target.wallpaper, found '{}'",
                untranslated_wallpaper,
            )
        );
    }


    let untranslated_app_name =
        spanish.text(
            connection,
            "app.name",
        )?;

    if untranslated_app_name
        != "Screenshaver"
    {
        return Err(
            format!(
                "Expected per-key English fallback for app.name, found '{}'",
                untranslated_app_name,
            )
        );
    }


    println!(
        "[LOCALIZATION TEST] es-US target.screensaver = {}",
        translated_screensaver
    );

    println!(
        "[LOCALIZATION TEST] es-US target.wallpaper = {}",
        untranslated_wallpaper
    );

    println!(
        "[LOCALIZATION TEST] es-US app.name = {}",
        untranslated_app_name
    );

    println!(
        "[LOCALIZATION TEST] Verified: es-US translation overrides canonical English"
    );

    println!(
        "[LOCALIZATION TEST] Verified: missing es-US keys fall back individually to canonical English"
    );

    println!(
        "[LOCALIZATION TEST] PASS"
    );


    Ok(())
}
