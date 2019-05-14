use juniper::{RootNode, EmptyMutation, FieldResult, graphql_object, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(description = "A Text item with suggestions")]
pub struct Item {
    pub id: String,
    pub text: String,
    pub suggestions: Vec<String>,
}

pub struct QueryRoot;

graphql_object!(QueryRoot: () |&self| {
    field item(&executor, id: String) -> FieldResult<Item> {
        Ok(Item {
            id: "324".to_owned(),
            text: "gurble".to_owned(),
            suggestions: vec!["garble".to_owned(), "groble".to_owned()],
        })
    }
});

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<()>>;

pub fn create_schema() -> Schema {
     Schema::new(QueryRoot {}, EmptyMutation::new())
}