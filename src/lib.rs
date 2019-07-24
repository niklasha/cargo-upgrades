use cargo_metadata::Dependency;
use crates_index::Crate;
use cargo_metadata::PackageId;
use cargo_metadata::Package;
use std::collections::HashMap;
use semver::Version;
use quick_error::quick_error;
pub use crates_index::Error as IndexError;
pub use cargo_metadata::Error as MetadataError;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Index(err: IndexError) {
            from()
            display("Can't fetch index: {}", err)
        }
        Metadata(err: MetadataError) {
            from()
            display("Can't get crate metadata: {}", err)
        }
    }
}

pub struct UpgradesChecker {
    workspace: Workspace,
    crates: HashMap<Box<str>, Crate>,
}

struct Workspace {
    packages: HashMap<PackageId, Package>,
    members: Vec<PackageId>,
}

impl UpgradesChecker {
    pub fn new(manifest_path: Option<&str>) -> Result<Self, Error> {
        let crates = std::thread::spawn(|| {
            let index = crates_index::Index::new_cargo_default();
            index.retrieve_or_update()?;
            let mut crates = HashMap::with_capacity(40000);
            for c in index.crates() {
                crates.insert(c.name().to_string().into_boxed_str(), c);
            }
            Ok(crates)
        });

        let workspace = Workspace::new(manifest_path)?;
        let crates: Result<_, IndexError> = crates.join().unwrap();

        Ok(Self {
            workspace,
            crates: crates?,
        })
    }
}

pub struct Match<'a> {
    pub dependency: &'a Dependency,
    pub matches: Option<Version>,
    pub latest: Version,
}

impl Workspace {
    pub fn new(manifest_path: Option<&str>) -> Result<Self, MetadataError> {
        let mut cmd = cargo_metadata::MetadataCommand::new();
        if let Some(path) = manifest_path {
            cmd.manifest_path(path);
        }
        let metadata = cmd.exec()?;
        Ok(Self {
            packages: metadata.packages.into_iter().map(|p| (p.id.clone(), p)).collect(),
            members: metadata.workspace_members,
        })
    }

    pub fn check_package(&self, id: &PackageId, index: &HashMap<Box<str>, Crate>) -> Option<(&Package, Vec<Match>)> {
        let package = self.packages.get(id)?;
        let deps = package.dependencies.iter().filter_map(|dep| {
            let is_from_crates_io = dep.source.as_ref().map_or(false, |d| d == "registry+https://github.com/rust-lang/crates.io-index");
            if !is_from_crates_io {
                return None;
            }
            let c = index.get(dep.name.as_str())?;
            let versions: Vec<_> = c.versions().iter().filter_map(|v| Version::parse(v.version()).ok()).collect();
            let latest_stable = versions.iter().filter(|v| v.pre.is_empty()).max();
            let latest_unstable = versions.iter().filter(|v| !v.pre.is_empty()).max();
            let latest_usable = latest_stable.or(latest_unstable)?;

            let latest_matching = versions.iter().filter(|v| dep.req.matches(v)).max();
            if latest_matching >= Some(latest_usable) {
                return None;
            }
            Some(Match {
                dependency: dep,
                matches: latest_matching.cloned(),
                latest: latest_usable.clone(),
            })
        })
        .collect();
        Some((package, deps))
    }
}

impl UpgradesChecker {
    pub fn outdated_dependencies<'a>(&'a self) -> impl Iterator<Item=(&Package, Vec<Match>)> + 'a {
        self.workspace.members.iter().filter_map(move |id| {
            self.workspace.check_package(id, &self.crates)
        })
        .filter(|(_, deps)| !deps.is_empty())
    }
}

#[test]
fn beta_vs_stable() {
    let beta11 = Version::parse("1.0.1-beta.1").unwrap();
    let beta1 = Version::parse("1.0.0-beta.1").unwrap();
    let v100 = Version::parse("1.0.0").unwrap();
    assert!(v100 > beta1);
    assert!(beta11 > beta1);
    assert!(beta11 > v100);
}

#[test]
fn test_self() {
    let u = UpgradesChecker::new(None).unwrap();
    assert_eq!(0, u.outdated_dependencies().count());
}
