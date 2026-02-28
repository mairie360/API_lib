pub mod utils;
// pub use utils::get_boolean_from_query_result;
// pub use utils::get_json_from_query_result;
// pub use utils::get_result_from_query_result;
// pub use utils::get_u64_from_query_result;
// pub use utils::QueryResult;

mod about_user_query_result_view;
pub use about_user_query_result_view::AboutUserQueryResultView;

mod does_user_exist_by_email;
pub use does_user_exist_by_email::DoesUserExistByEmailQueryResultView;

mod register_user;
pub use register_user::RegisterUserQueryResultView;

mod login_user;
pub use login_user::LoginUserQueryResultView;

mod does_user_exist_by_id;
pub use does_user_exist_by_id::DoesUserExistByIdQueryResultView;
