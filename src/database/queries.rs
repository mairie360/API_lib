use std::fmt::Display;

/**
 * This module defines the queries used in the database.
 * Each query is represented as an enum variant.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QUERY {
    AboutUser,
    DoesUserExistByEmail,
    DoesUserExistById,
    RegisterUser,
    LoginUser,
    UnknownQuery,
}

impl Display for QUERY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QUERY::AboutUser => write!(f, "AboutUser"),
            QUERY::DoesUserExistByEmail => write!(f, "DoesUserExistByEmail"),
            QUERY::DoesUserExistById => write!(f, "DoesUserExistById"),
            QUERY::RegisterUser => write!(f, "RegisterUser"),
            QUERY::LoginUser => write!(f, "LoginUser"),
            QUERY::UnknownQuery => write!(f, "UnknownQuery"),
        }
    }
}
