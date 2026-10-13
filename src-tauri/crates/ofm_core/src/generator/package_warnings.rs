//! Things a package author should hear about that do not stop the package loading.
//!
//! Errors (`validate_package`) say the package cannot be used. A warning says it can, but the
//! game will not do what the author probably meant. Today there is one: a country the package
//! defines itself, outside the nation catalog, with no `confederation`. Nothing places it, so
//! `nations::region_for_code` quietly files it, and its clubs, under Europe.
//!
//! (A club or player naming a country that neither the catalog nor the package defines is
//! already an error, `unknownCountry`.)

use super::package::{PackageError, WorldPackage};
use crate::nations::nation_by_code;

pub(super) const COUNTRY_WITHOUT_REGION: &str = "be.warning.package.countryWithoutRegion";

/// Problems that leave the package usable. Empty for a package whose own countries are all in
/// the catalog or placed in a confederation.
pub fn package_warnings(package: &WorldPackage) -> Vec<PackageError> {
    package
        .countries
        .iter()
        .enumerate()
        .filter(|(_, country)| {
            !country.id.trim().is_empty()
                && country.confederation.trim().is_empty()
                && nation_by_code(&country.id).is_none()
        })
        .map(|(index, country)| {
            PackageError::new(COUNTRY_WITHOUT_REGION, &package.source_at("country", index))
                .with("name", country.name.clone())
                .with("code", country.id.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::load_world_package;
    use std::path::{Path, PathBuf};

    fn package_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ofm-warn-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn warnings_for(tag: &str, files: &[(&str, &str)]) -> Vec<PackageError> {
        let dir = package_dir(tag);
        for (rel, contents) in files {
            write(&dir, rel, contents);
        }
        let (package, _errors) = load_world_package(&dir);
        std::fs::remove_dir_all(&dir).ok();
        package_warnings(&package)
    }

    const COUNTRY_QQ: (&str, &str) = (
        "countries/qq.json",
        r#"{"schema":"country","id":"QQ","name":"Qland"}"#,
    );

    /// Given a package that defines a country outside the catalog and puts it in no confederation
    /// When its warnings are listed
    /// Then one names the country, the file and the code.
    #[test]
    fn a_country_with_no_region_is_warned_about() {
        let warnings = warnings_for("qq", &[COUNTRY_QQ]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].code, COUNTRY_WITHOUT_REGION);
        assert_eq!(warnings[0].file, "countries/qq.json");
        assert!(
            warnings[0]
                .params
                .contains(&("name".into(), "Qland".into()))
        );
        assert!(warnings[0].params.contains(&("code".into(), "QQ".into())));
    }

    /// Given the same country placed in a confederation the package defines
    /// When its warnings are listed
    /// Then there is none: the package placed it.
    #[test]
    fn a_country_placed_in_a_confederation_is_silent() {
        let warnings = warnings_for(
            "placed",
            &[
                (
                    "countries/qq.json",
                    r#"{"schema":"country","id":"QQ","name":"Qland","confederation":"qconf"}"#,
                ),
                (
                    "confederations/qconf.json",
                    r#"{"schema":"confederation","id":"qconf","name":"Q Confed"}"#,
                ),
            ],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Given a package whose countries are all in the catalog, placed or not
    /// When it is validated
    /// Then there is no warning.
    #[test]
    fn a_catalog_country_is_silent() {
        let warnings = warnings_for(
            "catalog",
            &[(
                "countries/eng.json",
                r#"{"schema":"country","id":"ENG","name":"England"}"#,
            )],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Given a package that defines no country
    /// When it is validated
    /// Then there is no warning.
    #[test]
    fn a_package_without_countries_is_silent() {
        assert!(warnings_for("none", &[]).is_empty());
    }
}
