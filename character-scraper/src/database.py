from pymongo import MongoClient
from pymongo.errors import ConnectionError

class Database:
    def __init__(self, uri, db_name):
        self.client = None
        self.db = None
        self.connect(uri, db_name)

    def connect(self, uri, db_name):
        try:
            self.client = MongoClient(uri)
            self.db = self.client[db_name]
            print("Database connection successful.")
        except ConnectionError as e:
            print(f"Database connection failed: {e}")

    def insert_character(self, character_data):
        try:
            result = self.db.characters.insert_one(character_data)
            return result.inserted_id
        except Exception as e:
            print(f"Error inserting character: {e}")
            return None

    def update_character(self, character_id, update_data):
        try:
            result = self.db.characters.update_one({"_id": character_id}, {"$set": update_data})
            return result.modified_count
        except Exception as e:
            print(f"Error updating character: {e}")
            return 0

    def get_character(self, character_id):
        try:
            character = self.db.characters.find_one({"_id": character_id})
            return character
        except Exception as e:
            print(f"Error retrieving character: {e}")
            return None

    def close(self):
        if self.client:
            self.client.close()