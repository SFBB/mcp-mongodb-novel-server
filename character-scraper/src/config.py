import os

class Config:
    def __init__(self):
        self.target_url = os.getenv('TARGET_URL', 'https://example.com/characters')
        self.db_host = os.getenv('DB_HOST', 'localhost')
        self.db_port = int(os.getenv('DB_PORT', 27017))
        self.db_name = os.getenv('DB_NAME', 'character_db')
        self.scrape_interval = int(os.getenv('SCRAPE_INTERVAL', 3600))  # in seconds

    def get_database_uri(self):
        return f'mongodb://{self.db_host}:{self.db_port}/{self.db_name}'