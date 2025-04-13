# Character Information Scraper

This project is a character information scraper designed to fetch and store character details in a MongoDB database. It is structured to provide a clear separation of concerns, with dedicated modules for scraping, database interactions, and configuration management.

## Project Structure

```
character-scraper
├── src
│   ├── scraper.py        # Main logic for scraping character information
│   ├── database.py       # Handles MongoDB database connections and operations
│   └── config.py         # Manages configuration settings for the scraper
├── config
│   └── settings.json     # Configuration settings in JSON format
├── requirements.txt      # Python dependencies required for the project
└── README.md             # Documentation for the project
```

## Installation

1. Clone the repository:
   ```
   git clone <repository-url>
   cd character-scraper
   ```

2. Install the required dependencies:
   ```
   pip install -r requirements.txt
   ```

3. Configure the settings in `config/settings.json` to include your MongoDB credentials and any other necessary parameters.

## Usage

To run the scraper, execute the following command:
```
python src/scraper.py
```

This will initiate the scraping process, fetching character information from the specified source and storing it in the MongoDB database.

## Features

- Fetches character information from a specified source (HTML or JSON).
- Parses and extracts relevant character details.
- Stores and manages character data in a MongoDB database.
- Configurable settings for easy customization.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue for any enhancements or bug fixes.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.