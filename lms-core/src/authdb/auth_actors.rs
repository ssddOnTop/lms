use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub enum Authority {
    Admin,
    Faculty,
    #[default]
    Student,
}

impl FromStr for Authority {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();
        match s.as_str() {
            "admin" => Ok(Authority::Admin),
            "faculty" => Ok(Authority::Faculty),
            "student" => Ok(Authority::Student),
            _ => Err(anyhow!("Unable to serialize")),
        }
    }
}

impl Authority {
    pub fn from_int(int: u8) -> anyhow::Result<Self> {
        match int {
            0 => Ok(Authority::Admin),
            1 => Ok(Authority::Faculty),
            2 => Ok(Authority::Student),
            _ => Err(anyhow!("Unable to determine Authority")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub username: String,
    pub name: String,
    pub password: String,
    pub authority: Authority,
    pub batch: Option<String>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Users {
    users: HashMap<String, User>,
}

impl Users {
    pub fn get_all(&self) -> &HashMap<String, User> {
        &self.users
    }
    pub fn get(&self, username: &str) -> Option<User> {
        self.users.get(username).cloned()
    }
    pub fn insert(&mut self, user: User) {
        self.users.insert(user.username.clone(), user);
    }
    pub fn delete(&mut self, username: &str) {
        self.users.remove(username);
    }
}

#[cfg(test)]
mod tests {
    use crate::authdb::auth_actors::{Authority, User, Users};
    use std::collections::HashMap;

    #[test]
    fn test_ser() -> anyhow::Result<()> {
        let mut users = HashMap::new();
        users.insert(
            "foo".to_string(),
            User {
                username: "foo".to_string(),
                name: "Foo".to_string(),
                password: "foopassword".to_string(),
                authority: Authority::Admin,
                batch: Some("22BCS".to_string()),
            },
        );
        let users = Users { users };

        let ser = serde_json::to_string(&users)?;
        assert_eq!(
            ser,
            "{\"users\":{\"foo\":{\"username\":\"foo\",\"name\":\"Foo\",\"password\":\"foopassword\",\"authority\":\"Admin\",\"batch\":\"22BCS\"}}}"
        );
        Ok(())
    }
    #[test]
    fn test_deser() -> anyhow::Result<()> {
        let ser = r#"{"users":{"foo":{"username":"foo","name":"Foo","password":"foopassword","authority":"Admin","batch": "22BCS"}}}"#;
        let mut users = HashMap::new();
        users.insert(
            "foo".to_string(),
            User {
                username: "foo".to_string(),
                name: "Foo".to_string(),
                password: "foopassword".to_string(),
                authority: Authority::Admin,
                batch: Some("22BCS".to_string()),
            },
        );
        let users = Users { users };
        assert_eq!(users, serde_json::from_str(ser)?);
        Ok(())
    }
}
