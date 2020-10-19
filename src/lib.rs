pub use cargo_metadata::Error as MetadataError;
pub use crates_index::Error as IndexError;
use cargo_metadata::CargoOpt;
use cargo_metadata::Dependency;
use cargo_metadata::Metadata;
use cargo_metadata::MetadataCommand;
use cargo_metadata::Package;
use cargo_metadata::PackageId;
use crates_index::BareIndex as Index;
use crates_index::BareIndexRepo;
use quick_error::quick_error;
use semver::Version;
use std::collections::HashMap;

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

pub struct UpgradesCheckerInit {
    workspace: Workspace,
    index: Index,
}

impl UpgradesCheckerInit {
    pub fn new(manifest_path: Option<&str>) -> Result<Self, Error> {
        let crates = std::thread::spawn(|| {
            let index = Index::new_cargo_default();
            Ok(index)
        });

        let workspace = Workspace::new(manifest_path)?;
        let index: Result<_, IndexError> = crates.join().unwrap();

        Ok(Self {
            workspace,
            index: index?,
        })
    }

    pub fn checker(&self) -> Result<UpgradesChecker<'_>, Error> {
        Ok(UpgradesChecker {
            workspace: &self.workspace,
            index: self.index.open_or_clone()?,
        })
    }
}

pub struct UpgradesChecker<'a> {
    workspace: &'a Workspace,
    index: BareIndexRepo<'a>,
}

struct Workspace {
    packages: HashMap<PackageId, Package>,
    members: Vec<PackageId>,
}

pub struct Match<'a> {
    pub dependency: &'a Dependency,
    pub matches: Option<Version>,
    pub latest: Version,
}

impl Workspace {
    pub fn new(manifest_path: Option<&str>) -> Result<Self, MetadataError> {
        let metadata = Self::new_metadata(manifest_path, CargoOpt::AllFeatures)
        .or_else(|e| {
            Self::new_metadata(manifest_path, CargoOpt::SomeFeatures(vec![]))
            .or_else(|_| Self::new_metadata(manifest_path, CargoOpt::NoDefaultFeatures))
            .map_err(|_| e)
        })?;
        Ok(Self {
            packages: metadata.packages.into_iter().map(|p| (p.id.clone(), p)).collect(),
            members: metadata.workspace_members,
        })
    }

    fn new_metadata(manifest_path: Option<&str>, features: CargoOpt) -> Result<Metadata, MetadataError> {
        let mut cmd = MetadataCommand::new();
        if let Some(path) = manifest_path {
            cmd.manifest_path(path);
        }
        cmd.features(features);
        cmd.exec()
    }

    pub fn check_package(&self, id: &PackageId, index: &BareIndexRepo) -> Option<(&Package, Vec<Match>)> {
        let package = self.packages.get(id)?;
        let deps = package.dependencies.iter().filter_map(|dep| {
            let is_from_crates_io = dep.source.as_ref().map_or(false, |d| d == "registry+https://github.com/rust-lang/crates.io-index");
            if !is_from_crates_io {
                return None;
            }
            let c = index.crate_(dep.name.as_str())?;
            let versions: Vec<_> = c.versions().iter()
                .filter(|v| !v.is_yanked())
                .filter_map(|v| Version::parse(v.version()).ok())
                .collect();
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

impl UpgradesChecker<'_> {
    pub fn outdated_dependencies<'a>(&'a self) -> impl Iterator<Item=(&Package, Vec<Match>)> + 'a {
        self.workspace.members.iter().filter_map(move |id| {
            self.workspace.check_package(id, &self.index)
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
    let u = UpgradesCheckerInit::new(None).unwrap();
    let u = u.checker().unwrap();
    assert_eq!(0, u.outdated_dependencies().count());
}
