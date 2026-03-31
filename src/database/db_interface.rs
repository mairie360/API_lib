use crate::database::errors::DatabaseError;

pub trait QueryResultView {
    type Output;

    fn get_result(&self) -> Result<Self::Output, DatabaseError>;
}

pub trait DatabaseQueryView: Send {
    fn get_request(&self) -> String;
}
