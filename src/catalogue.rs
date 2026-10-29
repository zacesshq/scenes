//! Every scene the program can show: the built-in ones and the user's own Lua scenes from the
//! scenes folder beside the config file. The command line, the config, the watch mode and the
//! standalone mode all pick from here, so a user scene works everywhere a built-in one does.

use std::path::Path;

use crate::lua::{self, UserScene};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::scenes::{self, Kind};

/// One scene in the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub summary: String,
    pub kind: Kind,
    pub captions: Vec<String>,
    /// The index of the user scene, `None` for a built-in one.
    pub user: Option<usize>,
}

/// The scenes on offer and any user scenes that failed to load.
#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    pub entries: Vec<Entry>,
    pub users: Vec<UserScene>,
    /// Why a user scene could not be loaded, one line each.
    pub errors: Vec<String>,
}

impl Catalogue {
    /// The built-in scenes only.
    pub fn built_in() -> Self {
        let entries = scenes::SCENES
            .iter()
            .map(|info| Entry {
                name: info.name.to_string(),
                summary: info.summary.to_string(),
                kind: info.kind,
                captions: info.captions.iter().map(|c| c.to_string()).collect(),
                user: None,
            })
            .collect();
        Self {
            entries,
            users: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// The built-in scenes plus every user scene in `dir`, when there is one.
    pub fn load(dir: Option<&Path>) -> Self {
        let mut catalogue = Self::built_in();
        if let Some(dir) = dir {
            let (users, errors) = lua::load_dir(dir);
            catalogue.add_users(users);
            catalogue.errors = errors;
        }
        catalogue
    }

    fn add_users(&mut self, users: Vec<UserScene>) {
        for user in users {
            self.entries.push(Entry {
                name: user.name.clone(),
                summary: user.summary.clone(),
                kind: user.kind,
                captions: user.captions.clone(),
                user: Some(self.users.len()),
            });
            self.users.push(user);
        }
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Every scene that can stand for a passing build or every fail scene.
    pub fn names_for(&self, passing: bool) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.kind.passes() == passing)
            .map(|e| e.name.clone())
            .collect()
    }

    /// Every scene name, in order.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    /// Checks a list of scene names for the pass or the fail list. `what` names the source for
    /// the message, such as `--scene`.
    pub fn check_list(&self, names: &[String], passing: bool, what: &str) -> Result<(), String> {
        for name in names {
            let Some(entry) = self.find(name) else {
                return Err(format!(
                    "{what} names \"{name}\", which is not a scene. Run zacess-scenes list to see them."
                ));
            };
            if entry.kind.passes() != passing {
                let wanted = if passing { "pass or general" } else { "fail" };
                return Err(format!(
                    "{what} names {name}, a {} scene; it takes {wanted} scenes.",
                    entry.kind.name()
                ));
            }
        }
        Ok(())
    }

    /// Builds a scene by name. A user scene whose script fails is an error.
    pub fn create(
        &self,
        name: &str,
        width: usize,
        height: usize,
        motion: Motion,
        rng: &mut Rng,
    ) -> Result<Box<dyn Scene>, String> {
        let entry = self
            .find(name)
            .ok_or_else(|| format!("\"{name}\" is not a scene"))?;
        match entry.user {
            Some(i) => lua::create(&self.users[i], width, height, motion, rng),
            None => scenes::create(name, width, height, motion, rng)
                .ok_or_else(|| format!("\"{name}\" is not a scene")),
        }
    }

    /// Takes a scene out of the catalogue for the rest of the run, after its script failed.
    pub fn retire(&mut self, name: &str) {
        self.entries.retain(|e| e.name != name || e.user.is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_catalogue_has_every_scene() {
        let catalogue = Catalogue::built_in();
        assert_eq!(catalogue.entries.len(), scenes::SCENES.len());
        assert!(catalogue.names_for(true).contains(&"waves".to_string()));
        assert!(catalogue.names_for(false).contains(&"storm".to_string()));
        assert!(!catalogue.names_for(true).contains(&"storm".to_string()));
        let mut rng = Rng::new(1);
        assert!(
            catalogue
                .create("orbit", 20, 8, Motion::FULL, &mut rng)
                .is_ok()
        );
        assert!(
            catalogue
                .create("lava", 20, 8, Motion::FULL, &mut rng)
                .is_err()
        );
    }

    #[test]
    fn lists_are_checked_against_the_kinds() {
        let catalogue = Catalogue::built_in();
        let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(
            catalogue
                .check_list(&names(&["waves", "tea"]), true, "x")
                .is_ok()
        );
        assert!(catalogue.check_list(&names(&["flood"]), false, "x").is_ok());
        let error = catalogue
            .check_list(&names(&["flood"]), true, "--scene")
            .unwrap_err();
        assert!(
            error.contains("--scene") && error.contains("fail scene"),
            "{error}"
        );
        assert!(
            catalogue
                .check_list(&names(&["tea"]), false, "--fail-scene")
                .unwrap_err()
                .contains("takes fail scenes")
        );
        assert!(
            catalogue
                .check_list(&names(&["lava"]), true, "x")
                .unwrap_err()
                .contains("zacess-scenes list")
        );
    }

    #[test]
    fn user_scenes_join_the_catalogue_and_failures_are_reported() {
        let dir = std::env::temp_dir().join(format!("zacess-catalogue-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("my-lantern.lua"),
            include_str!("../examples/scenes/lantern.lua"),
        )
        .unwrap();
        std::fs::write(dir.join("broken.lua"), "return {").unwrap();
        let catalogue = Catalogue::load(Some(&dir));
        let user = catalogue
            .entries
            .iter()
            .find(|e| e.user.is_some())
            .expect("the lantern loads");
        assert_eq!(catalogue.errors.len(), 1, "{:?}", catalogue.errors);
        assert!(catalogue.errors[0].contains("broken.lua"));
        let mut rng = Rng::new(2);
        let name = user.name.clone();
        assert!(
            catalogue
                .create(&name, 40, 12, Motion::FULL, &mut rng)
                .is_ok()
        );
        let mut retired = catalogue.clone();
        retired.retire(&name);
        assert!(retired.find(&name).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
