use anyhow::Result;
use mongodb::{
    options::{ClientOptions, ResolverConfig},
    Client, Collection, Database,
};
use std::env;

#[derive(Clone)]
pub struct DatabaseConnection {
    client: Client,
    db: Database,
}

impl DatabaseConnection {
    pub async fn new() -> Result<Self> {
        // Load the MongoDB connection string from an environment variable
        let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");
        let db_name = env::var("DATABASE_NAME").expect("DATABASE_NAME must be set");

        // Create a ClientOptions instance and set the resolver config
        let options = ClientOptions::parse_with_resolver_config(&uri, ResolverConfig::cloudflare())
            .await?;

        // Get a handle to the MongoDB client
        let client = Client::with_options(options)?;
        let db = client.database(&db_name);

        // Test the connection with a valid ping command
        client
            .database("admin")
            .run_command(mongodb::bson::doc! { "ping": 1 }, None)
            .await?;

        tracing::info!("Connected to MongoDB");

        Ok(Self { client, db })
    }

    pub fn get_collection<T>(&self, collection_name: &str) -> Collection<T> {
        self.db.collection(collection_name)
    }
}