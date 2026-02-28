mod about_user;
pub use about_user::AboutUserQuery;

mod does_user_exist_by_email;
pub use does_user_exist_by_email::DoesUserExistByEmailQuery;

mod register_user;
pub use register_user::RegisterUserQuery;

mod login_user;
pub use login_user::LoginUserQuery;

mod does_user_exist_by_id;
pub use does_user_exist_by_id::DoesUserExistByIdQuery;

pub mod errors;
pub use errors::QueryError;
