use serde_json;
use std::fmt;

/**
 * This module provides utility functions to handle query results in a database context.
 * It defines a `QueryResult` enum to encapsulate different types of results,
 */
#[derive(Debug, PartialEq)]
pub enum QueryResult {
    Boolean(bool),
    JSON(serde_json::Value),
    Result(Result<(), String>),
    U64(u64),
}

/**
 * This function takes a `QueryResult` and returns a boolean value.
 * If the result is of type `QueryResult::Boolean`, it returns the boolean value.
 * If the result is not of type `QueryResult::Boolean`, it prints an error message and returns `false`.
 * * # Arguments
 * * `result`: A `QueryResult` enum instance.
 * # Returns
 * * A boolean value extracted from the `QueryResult`.
 */
pub fn get_boolean_from_query_result(result: QueryResult) -> bool {
    match result {
        QueryResult::Boolean(b) => b,
        _ => {
            eprintln!("Expected QueryResult::Boolean");
            false
        }
    }
}

/**
 * This function takes a `QueryResult` and returns a `serde_json::Value`.
 * If the result is of type `QueryResult::JSON`, it returns the contained JSON value
 *
 * # Arguments
 * `result`: A `QueryResult` enum instance.
 *
 * # Returns
 * A `serde_json::Value` extracted from the `QueryResult`.
 */
pub fn get_json_from_query_result(result: QueryResult) -> serde_json::Value {
    match result {
        QueryResult::JSON(json) => json,
        _ => {
            eprintln!("Expected QueryResult::JSON");
            serde_json::Value::Null
        }
    }
}

/**
 * This function takes a `QueryResult` and returns a `Result<(), String>`.
 * If the result is of type `QueryResult::Result`, it returns the contained result.
 * If the result is not of type `QueryResult::Result`, it prints an error message and returns an error.
 * * # Arguments
 * * `result`: A `QueryResult` enum instance.
 * # Returns
 * * A `Result<(), String>` extracted from the `QueryResult`.
 */
pub fn get_result_from_query_result(result: QueryResult) -> Result<(), String> {
    match result {
        QueryResult::Result(res) => res,
        _ => {
            eprintln!("Expected QueryResult::Result");
            Err("Expected QueryResult::Result".to_string())
        }
    }
}

/**
 * This function takes a `QueryResult` and returns a `u64`.
 * If the result is of type `QueryResult::U64`, it returns the contained value.
 * If the result is not of type `QueryResult::U64`, it prints an error message and returns 0.
 * * # Arguments
 * * `result`: A `QueryResult` enum instance.
 * # Returns
 * * A `u64` extracted from the `QueryResult`.
 */
pub fn get_u64_from_query_result(result: QueryResult) -> u64 {
    match result {
        QueryResult::U64(i) => i,
        _ => {
            eprintln!("Expected QueryResult::U64");
            0
        }
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryResult::Boolean(b) => write!(f, "{}", b),
            QueryResult::JSON(json) => write!(f, "{}", json),
            QueryResult::Result(res) => match res {
                Ok(_) => write!(f, "Success"),
                Err(e) => write!(f, "Error: {}", e),
            },
            QueryResult::U64(i) => write!(f, "{}", i),
        }
    }
}
