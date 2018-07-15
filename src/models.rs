use super::schema::users;

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name = "users"]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub avatar: String,
}
