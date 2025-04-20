"""
Scraper Configuration for MCP MongoDB Server

This file contains configuration settings for the novel scrapers
used to populate the MCP MongoDB database.

Each scraper has a list of novel URLs to scrape.
"""

# API endpoint for the MCP MongoDB server
API_URL = "http://localhost:3001"

# Maximum number of chapters to scrape per novel
MAX_CHAPTERS = 50

# Delay between scraping operations (in seconds)
REQUEST_DELAY = 1.0

# Novels to scrape
# Format:
# "scraper_name": [
#    {"url": "https://example.com/novel/123", "name": "Novel Name"},
# ]
NOVELS_TO_SCRAPE = {
    "69shu": [
        {"url": "https://www.69shu.com/txt/0/730", "name": "Example 69shu Novel 1"},
        # Add more 69shu novels here
    ],
    
    "baobao88": [
        {"url": "https://www.baobao88.com/lishi/27224/", "name": "Example Baobao88 Novel 1"},
        # Add more baobao88 novels here
    ],
    
    "quanben": [
        {"url": "https://www.quanben.io/n/yishijiezhixiong/", "name": "Example Quanben Novel 1"},
        # Add more quanben novels here
    ],
    
    "syosetu": [
        {"url": "https://ncode.syosetu.com/n9669bk/", "name": "Example Syosetu Novel 1"},
        # Add more syosetu novels here
    ],
    
    "ximalaya": [
        {"url": "https://www.ximalaya.com/album/12576446", "name": "Example Ximalaya Novel 1"},
        # Add more ximalaya novels here
    ]
}

# Scheduling configuration (in hours)
SCHEDULE_INTERVAL = 24  # Run once per day

# Email notification settings (optional)
ENABLE_EMAIL_NOTIFICATIONS = False
EMAIL_CONFIG = {
    "smtp_server": "smtp.example.com",
    "smtp_port": 587,
    "username": "your_username",
    "password": "your_password",
    "from_email": "notifications@example.com",
    "to_email": "admin@example.com"
}