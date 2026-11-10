//! Writing an edited package back to the files it was read from.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::Serialize;
use serde::de::{self, DeserializeOwned, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, json};

use super::competition_def::CompetitionDefinition;
use super::definitions::{NamePool, NamesDefinition, TeamDef};
use super::file_format::parse_definition_str;
use super::package::{
    ConfederationDef, CountryDef, PlayerDef, StaffDef, WorldMetaDef, collect_data_files,
};
use super::scaffold::{EntityKind, manifest_json, names_json};

/// What the package editor saves. A collection that is `None` was not sent: its files are left
/// exactly as they are on disk. `Some(vec![])` is a collection the author emptied, and is written
/// as such; the two must never be confused, or an editor that sends only what changed would
/// delete whatever it did not send.
#[derive(Default)]
pub struct EditedPackage {
    pub meta: Option<WorldMetaDef>,
    pub confederations: Option<Vec<ConfederationDef>>,
    pub countries: Option<Vec<CountryDef>>,
    pub teams: Option<Vec<TeamDef>>,
    pub players: Option<Vec<PlayerDef>>,
    pub staff: Option<Vec<StaffDef>>,
    pub names: Option<NamesDefinition>,
    pub competitions: Option<Vec<CompetitionDefinition>>,
}

/// What a save wrote: the collections it acknowledges, and how many files it touched. A caller
/// marks a collection saved only if it is listed here.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SaveOutcome {
    /// The names of the collections written: `meta`, `confederations`, `countries`, `teams`,
    /// `players`, `staff`, `names`, `competitions`.
    pub saved: Vec<&'static str>,
    pub files_written: usize,
}

/// Write `edited` into the package at `dir`, each entity into the file it was read from.
///
/// A package may spread its entities over as many files as the author likes (`ofm-cli add` writes
/// one per entity), and the loader reads every data file under the package. Writing each
/// collection into one file per kind, as the editor used to, left the files it had read beside the
/// ones it wrote, so the next open read every entity twice. Here each entity goes back to the file
/// that declares its id (the first, when two do), a new one goes to its kind's conventional file,
/// and a file whose entities were all removed is written with none. An item of a rewritten file
/// that does not read as its kind is carried over as written: the loader skipped it, so the editor
/// never held it. A file that does not parse at all is never written.
///
/// Only the collections present in `edited` are written; the files of an absent one are not read
/// for rewriting, not staged and not renamed. A save of nothing writes nothing and succeeds.
///
/// Every file written is JSON, which a `.yaml` file also parses as, so an entity read from YAML
/// stays loadable; its comments and layout do not survive the save.
///
/// The files are written as a set: each is staged beside its target first, and only once every
/// one is staged are they renamed into place. A save that fails while staging (a full disk, a
/// file that cannot be created) leaves the package as it was. With entities in files of their own,
/// a save that stopped halfway would leave some files new and some old, and nothing would say so.
pub fn save_package_files(dir: &Path, edited: &EditedPackage) -> Result<SaveOutcome, String> {
    let mut plan = SavePlan::default();
    let layout = Layout::read(dir)?;
    let collection = |kind: EntityKind| Collection {
        dir,
        kind,
        layout: &layout,
    };
    if let Some(meta) = &edited.meta {
        let manifest = layout.manifest.as_deref().unwrap_or("package.json");
        plan.add(
            "meta",
            vec![Pending::new(dir, manifest, manifest_json(meta)?)],
        );
    }
    if let Some(items) = &edited.confederations {
        plan.add(
            "confederations",
            collection(EntityKind::Confederation).files(items, |c| &c.id)?,
        );
    }
    if let Some(items) = &edited.countries {
        plan.add(
            "countries",
            collection(EntityKind::Country).files(items, |c| &c.id)?,
        );
    }
    if let Some(items) = &edited.teams {
        plan.add(
            "teams",
            collection(EntityKind::Team).files(items, |t| &t.id)?,
        );
    }
    if let Some(items) = &edited.players {
        plan.add(
            "players",
            collection(EntityKind::Player).files(items, |p| &p.id)?,
        );
    }
    if let Some(items) = &edited.staff {
        plan.add(
            "staff",
            collection(EntityKind::Staff).files(items, |s| &s.id)?,
        );
    }
    if let Some(items) = &edited.competitions {
        plan.add(
            "competitions",
            collection(EntityKind::Competition).files(items, |c| &c.id)?,
        );
    }
    if let Some(names) = &edited.names {
        plan.add("names", names_files(dir, names, &layout)?);
    }
    let outcome = plan.outcome();
    write_all(plan.pending)?;
    Ok(outcome)
}

/// The files a save will write, and the collections they belong to.
#[derive(Default)]
struct SavePlan {
    pending: Vec<Pending>,
    saved: Vec<&'static str>,
}

impl SavePlan {
    fn add(&mut self, collection: &'static str, files: Vec<Pending>) {
        self.saved.push(collection);
        self.pending.extend(files);
    }

    fn outcome(&self) -> SaveOutcome {
        SaveOutcome {
            saved: self.saved.clone(),
            files_written: self.pending.len(),
        }
    }
}

/// A file the save will write: where, and what.
struct Pending {
    path: PathBuf,
    contents: Value,
}

impl Pending {
    fn new(dir: &Path, file: &str, contents: Value) -> Self {
        Self {
            path: dir.join(file),
            contents,
        }
    }

    /// The sibling a file is staged in: its own name plus `.tmp`, so two targets never share one
    /// (`a.json` and `a.yaml` both have a stem `a`), and the loader, which reads only `.json`,
    /// `.yaml` and `.yml` files, never reads it.
    fn staging(&self) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(".tmp");
        PathBuf::from(name)
    }
}

/// Write `pending` as a set: stage every file, then rename each into place. If staging fails, the
/// files staged so far are removed and no target has changed. (A rename that fails after that,
/// which only the filesystem can cause, leaves the files before it renamed.)
fn write_all(pending: Vec<Pending>) -> Result<(), String> {
    let mut staged: Vec<&Pending> = Vec::new();
    for file in &pending {
        if let Err(error) = stage(file) {
            for done in &staged {
                std::fs::remove_file(done.staging()).ok();
            }
            return Err(error);
        }
        staged.push(file);
    }
    for (index, file) in pending.iter().enumerate() {
        if let Err(error) = std::fs::rename(file.staging(), &file.path) {
            // The `.json.tmp` siblings left behind would be archived into a built `.ofm`.
            for rest in &pending[index..] {
                std::fs::remove_file(rest.staging()).ok();
            }
            return Err(error.to_string());
        }
    }
    Ok(())
}

fn stage(file: &Pending) -> Result<(), String> {
    if let Some(parent) = file.path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&file.contents).map_err(|e| e.to_string())?;
    std::fs::write(file.staging(), text).map_err(|e| e.to_string())
}

/// A data file read for its layout only: its schema, and the id of each entity it declares. Every
/// other field is skipped without being built, which is what keeps a save from parsing a large
/// package twice over.
#[derive(Deserialize)]
struct FileHead {
    schema: Option<String>,
    /// A file of a single entity has its id at the top level.
    id: Option<LenientId>,
    items: Option<Vec<ItemHead>>,
}

/// One item of a file, read for its id only. Any item reads as one, whatever its shape: a file the
/// loader reads must be read here too, or the entities the editor took from it would be written
/// to the kind's conventional file and declared twice on the next open.
struct ItemHead {
    id: Option<String>,
}

/// An id as the layout reads it: the string, or `None` for a value of any other type (which the
/// loader rejects, so no entity the editor holds has it).
struct LenientId(Option<String>);

impl<'de> Deserialize<'de> for ItemHead {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ItemHeadVisitor)
    }
}

impl<'de> Deserialize<'de> for LenientId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LenientIdVisitor)
    }
}

/// A map key, read only for whether it is `id`, without building it.
struct KeyIsId(bool);

impl<'de> Deserialize<'de> for KeyIsId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(KeyIsIdVisitor)
    }
}

struct ItemHeadVisitor;
struct LenientIdVisitor;
struct KeyIsIdVisitor;

/// Implements every non-map, non-string visit of a visitor as "not an id", draining sequences so
/// the deserializer stays in step.
macro_rules! anything_else {
    ($value:expr) => {
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok($value)
        }
    };
}

impl<'de> Visitor<'de> for ItemHeadVisitor {
    type Value = ItemHead;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an entity")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<ItemHead, A::Error> {
        let mut id = None;
        while let Some(KeyIsId(is_id)) = map.next_key::<KeyIsId>()? {
            if is_id {
                id = map.next_value::<LenientId>()?.0;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(ItemHead { id })
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<ItemHead, E> {
        Ok(ItemHead { id: None })
    }

    anything_else!(ItemHead { id: None });
}

impl<'de> Visitor<'de> for LenientIdVisitor {
    type Value = LenientId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an id")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<LenientId, E> {
        Ok(LenientId(Some(value.to_string())))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<LenientId, D::Error> {
        LenientId::deserialize(deserializer)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<LenientId, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(LenientId(None))
    }

    anything_else!(LenientId(None));
}

impl<'de> Visitor<'de> for KeyIsIdVisitor {
    type Value = KeyIsId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a key")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<KeyIsId, E> {
        Ok(KeyIsId(value == "id"))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<KeyIsId, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(KeyIsId(false))
    }

    anything_else!(KeyIsId(false));
}

/// A names file read for its pools, which must read as pools for the file to count: the loader
/// drops a names file whose pools do not, so the editor never holds them, and the save must not
/// write that file over with none.
#[derive(Deserialize)]
struct NamesFile {
    version: Option<u32>,
    description: Option<String>,
    pools: Option<HashMap<String, NamePool>>,
    items: Option<Vec<NamesItem>>,
}

#[derive(Deserialize)]
struct NamesItem {
    pools: HashMap<String, NamePool>,
}

/// One data file of a schema, and the id of each item it declares, in order (`None` for an item
/// without one).
struct FileIds {
    file: String,
    ids: Vec<Option<String>>,
}

/// Where a package on disk declares each of its entities, read in the loader's own order (data
/// files sorted by path), so the first file to declare an id is the one the loader read first.
#[derive(Default)]
struct Layout {
    /// The first file of schema `world`.
    manifest: Option<String>,
    /// Per schema, each file of that schema with its items' ids.
    files: HashMap<String, Vec<FileIds>>,
    /// Each country's name pool and the file it was read from; a later file's pool replaces an
    /// earlier one's, as in the loader.
    name_pools: BTreeMap<String, String>,
    /// Every file of schema `names`, with the version and description it gives, which a save
    /// keeps: each file of a split package says what it holds.
    names_files: BTreeMap<String, (u32, String)>,
}

impl Layout {
    fn read(dir: &Path) -> Result<Self, String> {
        let mut paths = Vec::new();
        collect_data_files(dir, &mut paths)
            .map_err(|_| "be.error.package.readFailed".to_string())?;
        paths.sort();
        let mut layout = Self::default();
        for path in paths {
            let file = path
                .strip_prefix(dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(head) = parse_definition_str::<FileHead>(&text) else {
                continue;
            };
            layout.add(file, head, &text);
        }
        Ok(layout)
    }

    fn add(&mut self, file: String, head: FileHead, text: &str) {
        let Some(schema) = head.schema else {
            return;
        };
        match schema.as_str() {
            "world" => {
                self.manifest.get_or_insert(file);
            }
            "names" => {
                let Ok(names) = parse_definition_str::<NamesFile>(text) else {
                    return;
                };
                // Top-level pools after the items' pools: within one file they win, as in the
                // loader.
                let item_pools = names
                    .items
                    .into_iter()
                    .flatten()
                    .flat_map(|item| item.pools);
                for (country, _) in item_pools.chain(names.pools.into_iter().flatten()) {
                    self.name_pools.insert(country, file.clone());
                }
                let own = (
                    names.version.unwrap_or_default(),
                    names.description.unwrap_or_default(),
                );
                self.names_files.insert(file, own);
            }
            _ => {
                let ids = match head.items {
                    Some(items) => items.into_iter().map(|item| item.id).collect(),
                    None => vec![head.id.and_then(|id| id.0)],
                };
                self.files
                    .entry(schema)
                    .or_default()
                    .push(FileIds { file, ids });
            }
        }
    }
}

/// One kind's entities, written back over the package's layout.
struct Collection<'a> {
    dir: &'a Path,
    kind: EntityKind,
    layout: &'a Layout,
}

impl Collection<'_> {
    /// The files `entities` are written back to: each to the first file that declares its id, the
    /// rest to the kind's conventional file. Every file of this kind is among them, so an entity
    /// removed in the editor leaves its file too.
    fn files<T: Serialize + DeserializeOwned>(
        &self,
        entities: &[T],
        id: impl Fn(&T) -> &String,
    ) -> Result<Vec<Pending>, String> {
        let conventional = self.conventional_file();
        let on_disk = self
            .layout
            .files
            .get(self.kind.schema_name())
            .map_or(&[][..], Vec::as_slice);
        let mut home: HashMap<&str, &str> = HashMap::new();
        let mut files: BTreeMap<&str, Vec<Value>> = BTreeMap::new();
        for FileIds { file, ids } in on_disk {
            for entity_id in ids.iter().flatten() {
                home.entry(entity_id.as_str()).or_insert(file.as_str());
            }
            files.entry(file.as_str()).or_default();
        }
        for entity in entities {
            let file = home
                .get(id(entity).as_str())
                .copied()
                .unwrap_or(conventional.as_str());
            let value = serde_json::to_value(entity).map_err(|e| e.to_string())?;
            files.entry(file).or_default().push(value);
        }
        // A kind with nothing on disk and nothing to write still gets its file, as a new project
        // always has: the editor's next open expects the folder it scaffolded.
        if files.is_empty() {
            files.insert(conventional.as_str(), Vec::new());
        }
        let held: HashSet<&str> = entities.iter().map(|entity| id(entity).as_str()).collect();
        let mut pending = Vec::new();
        for (file, mut items) in files {
            let path = self.dir.join(file);
            if let Some(read) = on_disk.iter().find(|read| read.file == file) {
                items.extend(unreadable_items::<T>(&path, file, &read.ids, &held)?);
            }
            let contents = json!({"schema": self.kind.schema_name(), "items": items});
            pending.push(Pending { path, contents });
        }
        Ok(pending)
    }

    fn conventional_file(&self) -> String {
        match self.kind.dir() {
            Some(folder) => format!("{folder}/{}", self.kind.file_name()),
            None => self.kind.file_name().to_string(),
        }
    }
}

/// The items of the file at `path` that do not read as a `T`, as written. The loader skipped them
/// (and reported each), so the editor never held them, and rewriting their file from what it held
/// would delete them.
///
/// Only an item whose id the editor does not hold can be one: the editor holds every entity the
/// loader read. So the common save, which removed nothing and broke nothing, reads no file twice;
/// a file with such an item is parsed whole once, and only those items are tried as a `T`. One that
/// reads is an entity the author removed, and stays removed.
///
/// "Reads" is the loader's own test, a YAML value tried as a `T` (`serde_yaml::from_value`), so an
/// item the loader rejected is never taken for one the author removed. An unreadable item that
/// cannot be written as JSON (a YAML-only value, such as a non-string key) stops the save rather
/// than being dropped.
fn unreadable_items<T: DeserializeOwned>(
    path: &Path,
    file: &str,
    ids: &[Option<String>],
    held: &HashSet<&str>,
) -> Result<Vec<Value>, String> {
    let candidates: Vec<usize> = ids
        .iter()
        .enumerate()
        .filter(|(_, id)| id.as_deref().is_none_or(|id| !held.contains(id)))
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let Some(document) = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| parse_definition_str::<serde_yaml::Value>(&text).ok())
    else {
        return Ok(Vec::new());
    };
    let items: Vec<serde_yaml::Value> = match document.get("items") {
        Some(serde_yaml::Value::Sequence(items)) => items.clone(),
        _ => vec![document],
    };
    candidates
        .into_iter()
        .filter_map(|index| items.get(index))
        .filter(|item| serde_yaml::from_value::<T>((*item).clone()).is_err())
        .map(|item| {
            serde_json::to_value(item)
                .map_err(|_| format!("be.error.package.saveWouldDropItem?file={file}"))
        })
        .collect()
}

/// The names files a save writes: each country's pool back to the file it was read from, a new
/// one to the conventional names file. Every names file is among them, so a pool removed in the
/// editor leaves its file too, and each keeps the version and description it gave.
fn names_files(
    dir: &Path,
    names: &NamesDefinition,
    layout: &Layout,
) -> Result<Vec<Pending>, String> {
    let conventional = format!(
        "{}/{}",
        EntityKind::Names.dir().unwrap_or("names"),
        EntityKind::Names.file_name()
    );
    let mut files: BTreeMap<String, NamesDefinition> = layout
        .names_files
        .iter()
        .map(|(file, (version, description))| {
            let own = NamesDefinition {
                version: *version,
                description: description.clone(),
                pools: HashMap::new(),
            };
            (file.clone(), own)
        })
        .collect();
    for (country, pool) in &names.pools {
        let file = layout
            .name_pools
            .get(country)
            .cloned()
            .unwrap_or_else(|| conventional.clone());
        files
            .entry(file)
            .or_insert_with(|| empty_names_like(names))
            .pools
            .insert(country.clone(), pool.clone());
    }
    if files.is_empty() {
        files.insert(conventional, empty_names_like(names));
    }
    files
        .into_iter()
        .map(|(file, definition)| Ok(Pending::new(dir, &file, names_json(&definition)?)))
        .collect()
}

/// A names definition with `names`'s version and description and no pools: for a names file
/// the package did not have.
fn empty_names_like(names: &NamesDefinition) -> NamesDefinition {
    NamesDefinition {
        version: names.version,
        description: names.description.clone(),
        pools: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::package::load_world_package_files;

    /// A package directory holding `files` (relative path, contents), under a fresh temp dir.
    fn package_dir(files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ofm-save-{}", uuid::Uuid::new_v4()));
        for (name, body) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir created");
            std::fs::write(path, body).expect("file written");
        }
        dir
    }

    const MANIFEST: &str =
        r#"{"schema":"world","id":"split","name":"Split","version":"1.0.0","license":"CC0-1.0"}"#;

    /// Given an unreadable entity directory, when a changed package is saved, then no file changes.
    #[cfg(unix)]
    #[test]
    fn saving_an_unreadable_project_leaves_its_files_unchanged() {
        use std::os::unix::fs::PermissionsExt;
        let dir = package_dir(&[("package.json", MANIFEST), ("players/p.json", "{}")]);
        let package = edited(&dir, |package| {
            package.meta.as_mut().unwrap().name = "Renamed".to_string();
        });
        let nested = dir.join("players");
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0)).unwrap();
        let result = save_package_files(&dir, &package);
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700)).unwrap();
        let manifest = std::fs::read_to_string(dir.join("package.json")).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(result.unwrap_err(), "be.error.package.readFailed");
        assert_eq!(manifest, MANIFEST);
    }

    fn team_file(ids: &[&str]) -> String {
        let items: Vec<String> = ids
            .iter()
            .map(|id| {
                format!(
                    r##"{{"id":"{id}","name":"{id}","city":"C","country":"ENG","colors":{{"primary":"#000000","secondary":"#ffffff"}}}}"##
                )
            })
            .collect();
        format!(r#"{{"schema":"team","items":[{}]}}"#, items.join(","))
    }

    /// The package at `dir` as the editor would save it unchanged, with `edit` applied first.
    fn edited(dir: &Path, edit: impl FnOnce(&mut EditedPackage)) -> EditedPackage {
        let (package, _) = load_world_package_files(dir);
        let mut edited = EditedPackage {
            meta: Some(package.meta.expect("a manifest")),
            confederations: Some(package.confederations),
            countries: Some(package.countries),
            teams: Some(package.teams),
            players: Some(package.players),
            staff: Some(package.staff),
            names: Some(package.names.unwrap_or(NamesDefinition {
                version: 1,
                description: String::new(),
                pools: HashMap::new(),
            })),
            competitions: Some(package.competitions),
        };
        edit(&mut edited);
        edited
    }

    /// Each team the package at `dir` declares, as (file, id), sorted.
    fn teams_by_file(dir: &Path) -> Vec<(String, String)> {
        let (package, _) = load_world_package_files(dir);
        let mut found: Vec<(String, String)> = package
            .teams
            .iter()
            .zip(package.sources_of("team"))
            .map(|(team, file)| (file.clone(), team.id.clone()))
            .collect();
        found.sort();
        found
    }

    /// Given a package whose clubs live in two files of their own
    /// When a club is added in the editor and the package is saved
    /// Then the new club goes to the conventional teams file and the others stay where they were
    #[test]
    fn a_new_club_goes_to_the_conventional_teams_file() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/fc-a.json", &team_file(&["fc-a"])),
            ("teams/fc-b.json", &team_file(&["fc-b"])),
        ]);
        let package = edited(&dir, |package| {
            let teams = package.teams.as_mut().expect("teams sent");
            let mut extra = teams[0].clone();
            extra.id = "fc-c".to_string();
            teams.push(extra);
        });

        save_package_files(&dir, &package).expect("the save succeeds");
        let layout = teams_by_file(&dir);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            layout,
            vec![
                ("teams/fc-a.json".to_string(), "fc-a".to_string()),
                ("teams/fc-b.json".to_string(), "fc-b".to_string()),
                ("teams/teams.json".to_string(), "fc-c".to_string()),
            ]
        );
    }

    /// Given a package whose clubs live in two files of their own
    /// When one club is deleted in the editor and the package is saved
    /// Then its file no longer declares it, and the next open has only the club that is left
    #[test]
    fn a_removed_club_leaves_its_file() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/fc-a.json", &team_file(&["fc-a"])),
            ("teams/fc-b.json", &team_file(&["fc-b"])),
        ]);
        let package = edited(&dir, |package| {
            package
                .teams
                .as_mut()
                .expect("teams sent")
                .retain(|team| team.id != "fc-b")
        });

        save_package_files(&dir, &package).expect("the save succeeds");
        let layout = teams_by_file(&dir);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            layout,
            vec![("teams/fc-a.json".to_string(), "fc-a".to_string())]
        );
    }

    /// Given a teams file holding one valid club and one the loader cannot read
    /// When the editor saves the package, holding only the club it could read
    /// Then the unreadable club is still in its file: the editor never saw it, so a save must not
    ///      delete it
    #[test]
    fn a_club_the_loader_could_not_read_survives_the_save() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            (
                "teams/clubs.json",
                &team_file(&["fc-a"]).replace("]}", r#",{"id":"fc-broken","name":5}]}"#),
            ),
        ]);
        let package = edited(&dir, |_| {});
        assert_eq!(
            package.teams.as_ref().map(Vec::len),
            Some(1),
            "only the readable club loads"
        );

        save_package_files(&dir, &package).expect("the save succeeds");
        let written =
            std::fs::read_to_string(dir.join("teams").join("clubs.json")).expect("readable");
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            written.contains("fc-a") && written.contains("fc-broken"),
            "{written}"
        );
    }

    /// Given a teams file whose items include a bare string and a club with a numeric id, beside a
    ///      valid club
    /// When the editor saves the package and opens it again
    /// Then the valid club is declared once, in its own file, and the two odd items are kept: the
    ///      file is still read as the layout of the club the editor took from it
    #[test]
    fn odd_items_do_not_send_their_files_clubs_to_the_conventional_file() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            (
                "teams/clubs.json",
                &team_file(&["fc-a"]).replace("]}", r#","junk",{"id":7,"name":"Seven"}]}"#),
            ),
        ]);
        let package = edited(&dir, |_| {});

        save_package_files(&dir, &package).expect("the save succeeds");
        let layout = teams_by_file(&dir);
        let written =
            std::fs::read_to_string(dir.join("teams").join("clubs.json")).expect("readable");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            layout,
            vec![("teams/clubs.json".to_string(), "fc-a".to_string())]
        );
        assert!(
            written.contains("junk") && written.contains("Seven"),
            "{written}"
        );
    }

    /// A package with one club, plus `extra` at `rel`, saved unchanged; the bytes of `rel` after.
    fn bytes_after_save(rel: &str, extra: &str) -> (String, String) {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/fc-a.json", &team_file(&["fc-a"])),
            (rel, extra),
        ]);
        let package = edited(&dir, |_| {});
        save_package_files(&dir, &package).expect("the save succeeds");
        let after = std::fs::read_to_string(dir.join(rel)).expect("still there");
        std::fs::remove_dir_all(&dir).ok();
        (extra.to_string(), after)
    }

    /// Given a package beside a teams file that does not parse at all
    /// When the package is saved
    /// Then that file is left byte for byte: the editor never read it, so it has nothing to write
    #[test]
    fn a_file_that_does_not_parse_is_left_byte_for_byte() {
        let (before, after) = bytes_after_save("teams/broken.json", "{ \"schema\": \"team\", ");
        assert_eq!(after, before);
    }

    /// Given a package beside a data file with no schema
    /// When the package is saved
    /// Then that file is left byte for byte: the loader reports it and reads nothing from it
    #[test]
    fn a_file_without_a_schema_is_left_byte_for_byte() {
        let (before, after) = bytes_after_save(
            "teams/notes.json",
            r#"{"items":[{"id":"fc-z","name":"Z"}]}"#,
        );
        assert_eq!(after, before);
    }

    /// Given a package beside a data file of a schema the game does not know
    /// When the package is saved
    /// Then that file is left byte for byte
    #[test]
    fn a_file_of_an_unknown_schema_is_left_byte_for_byte() {
        let (before, after) = bytes_after_save(
            "stadiums/wembley.json",
            r#"{"schema":"stadium","id":"wembley","capacity":90000}"#,
        );
        assert_eq!(after, before);
    }

    /// Given a package whose two names files each describe what they hold
    /// When the package is saved
    /// Then each keeps its own description, not the merged definition's
    #[test]
    fn each_names_file_keeps_its_own_description() {
        let pools = |country: &str, description: &str| {
            format!(
                r#"{{"schema":"names","version":2,"description":"{description}","pools":{{"{country}":{{"first_names":["A"],"last_names":["B"]}}}}}}"#
            )
        };
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("names/br.json", &pools("BR", "Brazilian names")),
            ("names/pt.json", &pools("PT", "Portuguese names")),
        ]);
        let package = edited(&dir, |_| {});

        save_package_files(&dir, &package).expect("the save succeeds");
        let read = |file: &str| std::fs::read_to_string(dir.join(file)).expect("readable");
        let (br, pt) = (read("names/br.json"), read("names/pt.json"));
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            br.contains("Brazilian names") && !br.contains("Portuguese"),
            "{br}"
        );
        assert!(
            pt.contains("Portuguese names") && !pt.contains("Brazilian"),
            "{pt}"
        );
    }

    /// Given a package whose manifest is renamed in the editor, and a teams file whose staging
    ///      sibling cannot be created (a folder already has its name)
    /// When the package is saved
    /// Then the save fails and the manifest is unchanged on disk, with no staged file left: the
    ///      files are written as a set, so a save that cannot write them all writes none
    #[test]
    fn a_save_that_cannot_stage_every_file_changes_none() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/teams.json", &team_file(&["fc-a"])),
        ]);
        std::fs::create_dir_all(dir.join("teams").join("teams.json.tmp")).expect("blocker made");
        let package = edited(&dir, |package| {
            package.meta.as_mut().expect("meta sent").name = "Renamed".to_string()
        });

        let result = save_package_files(&dir, &package);
        let manifest = std::fs::read_to_string(dir.join("package.json")).expect("readable");
        let staged_manifest = dir.join("package.json.tmp").exists();
        std::fs::remove_dir_all(&dir).ok();

        assert!(result.is_err(), "the save reports the failure");
        assert_eq!(manifest, MANIFEST);
        assert!(!staged_manifest, "the staged manifest is removed");
    }

    /// Given a package whose name pools for two countries live in two files
    /// When one pool is edited in the editor and the package is saved
    /// Then each pool is still in its own file, the edit is in the edited pool's file, and no
    ///      third names file appears
    #[test]
    fn each_name_pool_goes_back_to_its_file() {
        let pools = |country: &str, first: &str| {
            format!(
                r#"{{"schema":"names","pools":{{"{country}":{{"first_names":["{first}"],"last_names":["Silva"]}}}}}}"#
            )
        };
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("names/br.json", &pools("BR", "Joao")),
            ("names/pt.json", &pools("PT", "Rui")),
        ]);
        let package = edited(&dir, |package| {
            if let Some(pool) = package
                .names
                .as_mut()
                .expect("names sent")
                .pools
                .get_mut("BR")
            {
                pool.first_names = vec!["Pedro".to_string()];
            }
        });

        save_package_files(&dir, &package).expect("the save succeeds");
        let read = |file: &str| std::fs::read_to_string(dir.join(file)).expect("readable");
        let (br, pt) = (read("names/br.json"), read("names/pt.json"));
        let has_conventional = dir.join("names").join("names.json").exists();
        std::fs::remove_dir_all(&dir).ok();

        assert!(br.contains("Pedro") && !br.contains("\"PT\""), "{br}");
        assert!(pt.contains("Rui") && !pt.contains("\"BR\""), "{pt}");
        assert!(!has_conventional, "no third names file is created");
    }

    const PLAYERS: &str = r#"{"schema":"player","items":[]}"#;

    /// Every file under `dir` (relative path, bytes), sorted: the whole package on disk.
    fn files_under(dir: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, base: &Path, into: &mut Vec<(String, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).expect("readable").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, base, into);
                } else {
                    let name = path.strip_prefix(base).expect("under base");
                    into.push((
                        name.to_string_lossy().replace('\\', "/"),
                        std::fs::read(&path).expect("readable"),
                    ));
                }
            }
        }
        let mut found = Vec::new();
        walk(dir, dir, &mut found);
        found.sort();
        found
    }

    fn only_teams(dir: &Path, edit: impl FnOnce(&mut Vec<TeamDef>)) -> EditedPackage {
        let mut package = EditedPackage::default();
        let mut teams = edited(dir, |_| {}).teams.expect("teams");
        edit(&mut teams);
        package.teams = Some(teams);
        package
    }

    /// Given a package with clubs and players in files of their own
    /// When only the clubs are saved, with one added
    /// Then the players' file is byte for byte as it was, no staged file is left beside it, and the
    ///      save acknowledges the clubs only, naming the two files it wrote
    #[test]
    fn a_collection_that_was_not_sent_is_left_untouched() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/fc-a.json", &team_file(&["fc-a"])),
            ("players/p.json", PLAYERS),
        ]);
        let package = only_teams(&dir, |teams| {
            let mut extra = teams[0].clone();
            extra.id = "fc-c".to_string();
            teams.push(extra);
        });

        let outcome = save_package_files(&dir, &package).expect("the save succeeds");
        let after = files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();

        let read = |name: &str| {
            after
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b.clone())
        };
        assert_eq!(read("players/p.json"), Some(PLAYERS.as_bytes().to_vec()));
        assert_eq!(read("package.json"), Some(MANIFEST.as_bytes().to_vec()));
        assert!(
            after.iter().all(|(name, _)| !name.ends_with(".tmp")),
            "{after:?}"
        );
        assert_eq!(outcome.saved, vec!["teams"]);
        assert_eq!(outcome.files_written, 2);
    }

    /// Given a package with clubs and players
    /// When the clubs are sent as an empty list and the players are not sent
    /// Then the clubs are emptied, and the players are untouched: sent-and-empty is not omitted
    #[test]
    fn an_emptied_collection_is_written_and_an_omitted_one_is_not() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/teams.json", &team_file(&["fc-a", "fc-b"])),
            ("players/p.json", PLAYERS),
        ]);
        let package = only_teams(&dir, |teams| teams.clear());

        let outcome = save_package_files(&dir, &package).expect("the save succeeds");
        let teams = std::fs::read_to_string(dir.join("teams/teams.json")).expect("readable");
        let players = std::fs::read_to_string(dir.join("players/p.json")).expect("readable");
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            !teams.contains("fc-a") && !teams.contains("fc-b"),
            "{teams}"
        );
        assert_eq!(players, PLAYERS);
        assert_eq!(outcome.saved, vec!["teams"]);
    }

    /// Given a package
    /// When a save sends no collection at all
    /// Then it succeeds, writes no file and acknowledges nothing: a clean save costs nothing
    #[test]
    fn a_save_of_nothing_writes_nothing() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/teams.json", &team_file(&["fc-a"])),
        ]);
        let before = files_under(&dir);

        let outcome = save_package_files(&dir, &EditedPackage::default()).expect("succeeds");
        let after = files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(outcome, SaveOutcome::default());
        assert_eq!(after, before);
    }

    /// Given a package
    /// When only its manifest is sent, renamed
    /// Then the manifest changes and every other file is byte for byte as it was
    #[test]
    fn a_manifest_alone_leaves_every_entity_file_alone() {
        let dir = package_dir(&[
            ("package.json", MANIFEST),
            ("teams/teams.json", &team_file(&["fc-a"])),
            ("players/p.json", PLAYERS),
        ]);
        let before = files_under(&dir);
        let mut package = EditedPackage::default();
        let mut meta = edited(&dir, |_| {}).meta.expect("meta");
        meta.name = "Renamed".to_string();
        package.meta = Some(meta);

        let outcome = save_package_files(&dir, &package).expect("succeeds");
        let after = files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();

        let changed: Vec<&str> = after
            .iter()
            .zip(&before)
            .filter(|(a, b)| a != b)
            .map(|(a, _)| a.0.as_str())
            .collect();
        assert_eq!(changed, vec!["package.json"]);
        assert_eq!(outcome.saved, vec!["meta"]);
        assert_eq!(outcome.files_written, 1);
    }
}
