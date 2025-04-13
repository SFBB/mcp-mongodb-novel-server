# MCP MongoDB Database Interface Documentation

This document provides a detailed guide on how to interact with the MCP MongoDB database. It's designed to help developers create data population tools (like Python scrapers) or other applications that need to read from or write to the database.

## Database Schema

Our database is optimized for small context windows (3k tokens) and consists of the following collections:

### 1. Novels Collection

Stores basic information about novels with optional extended metadata:

```json
{
  "_id": "ObjectId",
  "title": "String",
  "author": "String",
  "summary": "String",
  "tags": ["String"],
  "metadata": {
    "publication_date": "String",
    "genre": ["String"],
    "word_count": "Number",
    "language": "String"
  }
}
```

### 2. Chapters Collection

Stores chapter information with a focus on summaries and key points:

```json
{
  "_id": "ObjectId",
  "novel_id": "ObjectId",
  "number": "Number",
  "title": "String",
  "summary": "String", 
  "key_points": ["String"],
  "content": "String" // Full chapter content, optional
}
```

### 3. Characters Collection

Stores character information with relationship mappings:

```json
{
  "_id": "ObjectId",
  "novel_id": "ObjectId",
  "name": "String",
  "role": "String", // "protagonist", "antagonist", "supporting"
  "description": "String",
  "key_traits": ["String"],
  "relationships": [
    {
      "character_id": "ObjectId", // Optional
      "character_name": "String", 
      "relationship_type": "String" // "friend", "enemy", "family", etc.
    }
  ]
}
```

### 4. QA Collection

Stores question-answer pairs for knowledge base:

```json
{
  "_id": "ObjectId",
  "novel_id": "ObjectId", // Optional, can be null for general Q&A
  "question": "String",
  "answer": "String",
  "tags": ["String"]
}
```

## MongoDB Indexes

For optimal performance, create the following indexes in your MongoDB database:

```javascript
// Novels collection
db.novels.createIndex({ "title": "text", "author": "text", "summary": "text", "tags": 1 });

// Chapters collection
db.chapters.createIndex({ "novel_id": 1 });
db.chapters.createIndex({ "novel_id": 1, "number": 1 }, { unique: true });
db.chapters.createIndex({ "title": "text", "summary": "text", "key_points": "text" });

// Characters collection
db.characters.createIndex({ "novel_id": 1 });
db.characters.createIndex({ "novel_id": 1, "name": 1 }, { unique: true });
db.characters.createIndex({ "name": "text", "description": "text", "key_traits": "text" });

// QA collection
db.qa.createIndex({ "novel_id": 1 });
db.qa.createIndex({ "tags": 1 });
db.qa.createIndex({ "question": "text", "answer": "text" });
```

## Using the CRUD APIs

### REST API Endpoints

The MCP server provides RESTful CRUD endpoints for each collection. Below are examples of how to interact with these endpoints using Python:

#### Python Client Example

```python
import requests
import json
from bson import ObjectId

# Helper to convert ObjectId to string for JSON serialization
class JSONEncoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, ObjectId):
            return str(o)
        return super().default(o)

BASE_URL = "http://localhost:3000"

# Novel operations
def create_novel(novel_data):
    response = requests.post(f"{BASE_URL}/api/novels", json=novel_data)
    return response.json()

def get_novel(novel_id):
    response = requests.get(f"{BASE_URL}/api/novels/{novel_id}")
    return response.json()

def update_novel(novel_id, update_data):
    response = requests.patch(f"{BASE_URL}/api/novels/{novel_id}", json=update_data)
    return response.json()

def delete_novel(novel_id):
    response = requests.delete(f"{BASE_URL}/api/novels/{novel_id}")
    return response.status_code == 204

# Similar functions for chapters, characters, and QA entries
# ...

# Example usage
novel = {
    "title": "The Great Adventure",
    "author": "Jane Smith",
    "summary": "An epic journey across magical lands.",
    "tags": ["fantasy", "adventure", "magic"]
}

# Create a novel
created_novel = create_novel(novel)
novel_id = created_novel["_id"]

# Add a chapter
chapter = {
    "novel_id": novel_id,
    "number": 1,
    "title": "The Beginning",
    "summary": "The protagonist discovers a mysterious map.",
    "key_points": ["Map discovery", "Meeting the mentor", "Decision to embark"],
    "content": "Once upon a time in a small village..."
}
create_chapter(chapter)

# Add a character
character = {
    "novel_id": novel_id,
    "name": "Alex Adventurer",
    "role": "protagonist",
    "description": "A brave young explorer with a keen sense of direction.",
    "key_traits": ["brave", "intelligent", "loyal"],
    "relationships": []
}
create_character(character)
```

### MongoDB Direct Access (Python Example)

For bulk imports or more complex operations, you might want to use the MongoDB driver directly:

```python
from pymongo import MongoClient
import datetime
from bson import ObjectId

# Connect to MongoDB
client = MongoClient('mongodb://localhost:27017/')
db = client['novel_database']

# Collections
novels = db.novels
chapters = db.chapters
characters = db.characters
qa = db.qa

# Bulk insert example
def import_novel_with_chapters(novel_data, chapters_data):
    # Insert the novel
    novel_id = novels.insert_one(novel_data).inserted_id
    
    # Add novel_id to each chapter and insert
    for chapter in chapters_data:
        chapter['novel_id'] = novel_id
    
    if chapters_data:
        chapters.insert_many(chapters_data)
    
    return novel_id

# Web scraping example
def scrape_novel_from_website(url):
    # This is a simplified example - in reality, you'd use requests and BeautifulSoup
    import requests
    from bs4 import BeautifulSoup
    
    response = requests.get(url)
    soup = BeautifulSoup(response.text, 'html.parser')
    
    # Extract novel information
    novel = {
        "title": soup.find('h1').text.strip(),
        "author": soup.find('div', class_='author').text.strip(),
        "summary": soup.find('div', class_='summary').text.strip(),
        "tags": [tag.text.strip() for tag in soup.find_all('span', class_='tag')],
        "metadata": {
            "publication_date": soup.find('div', class_='pub-date').text.strip(),
            "genre": [genre.text.strip() for genre in soup.find_all('span', class_='genre')]
        }
    }
    
    # Extract chapters
    chapters = []
    for chapter_elem in soup.find_all('div', class_='chapter'):
        chapter = {
            "number": int(chapter_elem.find('span', class_='number').text.strip()),
            "title": chapter_elem.find('h2').text.strip(),
            "summary": chapter_elem.find('div', class_='chapter-summary').text.strip(),
            "key_points": [point.text.strip() for point in chapter_elem.find_all('li', class_='key-point')],
            "content": chapter_elem.find('div', class_='content').text.strip()
        }
        chapters.append(chapter)
    
    # Import to database
    novel_id = import_novel_with_chapters(novel, chapters)
    
    # Extract characters
    for character_elem in soup.find_all('div', class_='character'):
        character = {
            "novel_id": novel_id,
            "name": character_elem.find('h3').text.strip(),
            "role": character_elem.find('span', class_='role').text.strip(),
            "description": character_elem.find('div', class_='description').text.strip(),
            "key_traits": [trait.text.strip() for trait in character_elem.find_all('span', class_='trait')]
        }
        characters.insert_one(character)
    
    return novel_id
```

## Best Practices for Data Population

When populating the database, consider these best practices:

1. **Optimize for Token Efficiency**: Keep string fields concise and information-dense to optimize for small context windows.

2. **Include Summaries**: Always provide good quality summaries for chapters and character descriptions, as these are key to the MCP server's ability to respond well with limited tokens.

3. **Use Key Points**: Break down complex information into key points for efficient retrieval.

4. **Consistent Relationships**: When adding relationships between characters, ensure the character names match exactly.

5. **Index Creation**: Before bulk imports, ensure all indexes are created for better performance.

6. **Batch Processing**: For large imports, use batch processing:

```python
def batch_import_chapters(novel_id, chapters_data, batch_size=100):
    for i in range(0, len(chapters_data), batch_size):
        batch = chapters_data[i:i+batch_size]
        for chapter in batch:
            chapter['novel_id'] = novel_id
        chapters.insert_many(batch)
```

7. **Text Search Optimization**: To improve text search results, consider adding keywords in the tags field.

8. **Data Validation**: Validate data before insertion to ensure consistency:

```python
def validate_novel(novel):
    required_fields = ['title', 'author', 'summary']
    for field in required_fields:
        if field not in novel or not novel[field]:
            raise ValueError(f"Missing required field: {field}")
    
    if 'tags' in novel and not isinstance(novel['tags'], list):
        raise ValueError("Tags must be a list")
    
    return True
```

## Schema and Index Setup Script

For convenience, here's a MongoDB script to set up your database schema and indexes:

```javascript
// Create database and collections
use novel_database;

db.createCollection("novels");
db.createCollection("chapters");
db.createCollection("characters");
db.createCollection("qa");

// Create indexes for novels collection
db.novels.createIndex({ "title": "text", "author": "text", "summary": "text" });
db.novels.createIndex({ "tags": 1 });

// Create indexes for chapters collection
db.chapters.createIndex({ "novel_id": 1 });
db.chapters.createIndex({ "novel_id": 1, "number": 1 }, { unique: true });
db.chapters.createIndex({ "title": "text", "summary": "text", "key_points": "text" });

// Create indexes for characters collection
db.characters.createIndex({ "novel_id": 1 });
db.characters.createIndex({ "novel_id": 1, "name": 1 }, { unique: true });
db.characters.createIndex({ "name": "text", "description": "text", "key_traits": "text" });

// Create indexes for QA collection
db.qa.createIndex({ "novel_id": 1 });
db.qa.createIndex({ "tags": 1 });
db.qa.createIndex({ "question": "text", "answer": "text" });
```

## Token Optimization Tips

Since our MCP server is optimized for small context windows (3k tokens), consider these tips:

1. **Character Descriptions**: Keep character descriptions under 100 words but include key defining traits.

2. **Chapter Summaries**: Aim for 50-150 word summaries that capture the essential plot points.

3. **Key Points**: Use 3-7 key points per chapter to highlight the most important events.

4. **Q&A Entries**: Structure Q&A entries to be specific and focused on a single concept.

5. **Tags**: Use consistent tagging to improve searchability without using too many tokens.

6. **Relationships**: Focus on important relationships that impact the narrative.

7. **Content Storage**: For full chapter content, consider storing it with compression or in chunks if extremely long.

By following these guidelines, you can create a well-structured database that works efficiently with the MCP server's optimization for small context windows.