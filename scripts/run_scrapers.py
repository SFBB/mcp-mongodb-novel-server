#!/usr/bin/env python
"""
Scraper Automation for MCP MongoDB Server

This script automates the scraping of novels from various sources 
and populates the MCP MongoDB database. It can be run manually or 
scheduled as a cron job.

Usage:
    python run_scrapers.py [--once] [--scraper SCRAPER_NAME] [--verbose]

Options:
    --once      Run once and exit (don't run on schedule)
    --scraper   Run only the specified scraper
    --verbose   Enable verbose logging
"""

import os
import sys
import time
import argparse
import logging
import smtplib
import json
import random
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple

# Add scraper_library to path
SCRIPT_DIR = Path(__file__).resolve().parent
SCRAPER_LIB_DIR = SCRIPT_DIR.parent / "scraper_library"
sys.path.append(str(SCRAPER_LIB_DIR))

# Import configuration
import scraper_config as config

# Import the scraper script
from scrape_novel import scrape_novel_to_database, SCRAPERS

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler(SCRIPT_DIR / 'scraper_automation.log')
    ]
)
logger = logging.getLogger("scraper_automation")

class ScraperAutomation:
    """Automates the scraping of novels from various sources."""
    
    def __init__(self, run_once: bool = False, specific_scraper: Optional[str] = None, verbose: bool = False):
        """
        Initialize the automation system.
        
        Args:
            run_once: Whether to run once and exit
            specific_scraper: Run only the specified scraper
            verbose: Enable verbose logging
        """
        self.run_once = run_once
        self.specific_scraper = specific_scraper
        
        # Set up logging level
        if verbose:
            logger.setLevel(logging.DEBUG)
        
        # Initialize statistics tracking
        self.stats = {
            "start_time": None,
            "end_time": None,
            "total_novels": 0,
            "successful_novels": 0,
            "failed_novels": 0,
            "scrapers_run": [],
            "novels_processed": []
        }
        
        # Database connection check
        self._check_database_connection()
    
    def _check_database_connection(self) -> bool:
        """Check if the MCP MongoDB server is available."""
        import requests
        
        try:
            response = requests.get(f"{config.API_URL}/api/novels", timeout=5)
            response.raise_for_status()
            logger.info(f"Successfully connected to MCP MongoDB server at {config.API_URL}")
            return True
        except requests.RequestException as e:
            logger.error(f"Failed to connect to MCP MongoDB server: {e}")
            logger.error(f"Make sure the server is running at {config.API_URL}")
            return False
    
    def _send_notification_email(self, subject: str, body: str) -> None:
        """Send a notification email with the scraping results."""
        if not config.ENABLE_EMAIL_NOTIFICATIONS:
            return
        
        try:
            msg = MIMEMultipart()
            msg['From'] = config.EMAIL_CONFIG['from_email']
            msg['To'] = config.EMAIL_CONFIG['to_email']
            msg['Subject'] = subject
            
            msg.attach(MIMEText(body, 'plain'))
            
            server = smtplib.SMTP(config.EMAIL_CONFIG['smtp_server'], config.EMAIL_CONFIG['smtp_port'])
            server.starttls()
            server.login(config.EMAIL_CONFIG['username'], config.EMAIL_CONFIG['password'])
            server.send_message(msg)
            server.quit()
            
            logger.info(f"Notification email sent to {config.EMAIL_CONFIG['to_email']}")
        except Exception as e:
            logger.error(f"Failed to send notification email: {e}")
    
    def _format_stats_report(self) -> str:
        """Format statistics into a readable report."""
        if not self.stats["start_time"]:
            return "No statistics available (scraping not started)"
        
        duration = (self.stats["end_time"] or datetime.now()) - self.stats["start_time"]
        
        report = [
            f"Scraping Report - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
            f"----------------------------------------------------------",
            f"Start time: {self.stats['start_time'].strftime('%Y-%m-%d %H:%M:%S')}",
            f"End time: {(self.stats['end_time'] or datetime.now()).strftime('%Y-%m-%d %H:%M:%S')}",
            f"Duration: {duration}",
            f"",
            f"Total novels: {self.stats['total_novels']}",
            f"Successful: {self.stats['successful_novels']}",
            f"Failed: {self.stats['failed_novels']}",
            f"",
            f"Scrapers run: {', '.join(self.stats['scrapers_run'])}",
            f"",
            f"Novels processed:"
        ]
        
        for novel in self.stats['novels_processed']:
            status = "✅ Success" if novel['success'] else "❌ Failed"
            report.append(f"  - {novel['name']} ({novel['scraper']}): {status}")
            if not novel['success']:
                report.append(f"    Error: {novel['error']}")
        
        return "\n".join(report)
    
    def _scrape_novel(self, scraper_name: str, novel_data: Dict) -> Tuple[bool, Optional[str], Optional[Dict]]:
        """
        Scrape a single novel and return the result.
        
        Args:
            scraper_name: Name of the scraper to use
            novel_data: Dictionary with novel URL and name
            
        Returns:
            Tuple of (success, error_message, result_data)
        """
        try:
            logger.info(f"Starting to scrape '{novel_data['name']}' using {scraper_name} scraper")
            
            # Scrape the novel
            result = scrape_novel_to_database(
                scraper_name=scraper_name,
                novel_url=novel_data['url'],
                api_url=config.API_URL
            )
            
            logger.info(f"Successfully scraped '{novel_data['name']}'")
            return True, None, result
        
        except Exception as e:
            logger.error(f"Failed to scrape '{novel_data['name']}': {str(e)}")
            return False, str(e), None
    
    def run(self) -> None:
        """Run the scraper automation process."""
        logger.info("Starting scraper automation")
        
        self.stats["start_time"] = datetime.now()
        
        try:
            # Determine which scrapers to run
            scrapers_to_run = {}
            
            if self.specific_scraper:
                # Only run the specified scraper
                if self.specific_scraper in config.NOVELS_TO_SCRAPE:
                    scrapers_to_run[self.specific_scraper] = config.NOVELS_TO_SCRAPE[self.specific_scraper]
                    logger.info(f"Running only the {self.specific_scraper} scraper")
                else:
                    logger.error(f"Specified scraper '{self.specific_scraper}' not found in configuration")
                    return
            else:
                # Run all scrapers
                scrapers_to_run = config.NOVELS_TO_SCRAPE
                logger.info(f"Running all scrapers: {', '.join(scrapers_to_run.keys())}")
            
            # Track total novels
            total_novels = sum(len(novels) for novels in scrapers_to_run.values())
            self.stats["total_novels"] = total_novels
            
            logger.info(f"Found {total_novels} novels to scrape")
            
            # Process each scraper
            for scraper_name, novels in scrapers_to_run.items():
                if not novels:
                    logger.info(f"No novels configured for {scraper_name}, skipping")
                    continue
                
                if scraper_name not in SCRAPERS:
                    logger.error(f"Scraper '{scraper_name}' not found in available scrapers")
                    continue
                
                logger.info(f"Running {scraper_name} scraper for {len(novels)} novels")
                self.stats["scrapers_run"].append(scraper_name)
                
                # Process each novel
                for novel in novels:
                    # Add some randomness to delay to be nicer to servers
                    jitter = random.uniform(0.5, 1.5)
                    time.sleep(config.REQUEST_DELAY * jitter)
                    
                    success, error, result = self._scrape_novel(scraper_name, novel)
                    
                    # Track statistics
                    novel_stats = {
                        "name": novel["name"],
                        "url": novel["url"],
                        "scraper": scraper_name,
                        "success": success,
                        "error": error,
                        "result": result
                    }
                    self.stats["novels_processed"].append(novel_stats)
                    
                    if success:
                        self.stats["successful_novels"] += 1
                    else:
                        self.stats["failed_novels"] += 1
            
            # Finalize statistics
            self.stats["end_time"] = datetime.now()
            
            # Log results
            logger.info(f"Scraping completed. Success: {self.stats['successful_novels']}, Failed: {self.stats['failed_novels']}")
            
            # Send notification email
            if config.ENABLE_EMAIL_NOTIFICATIONS:
                subject = f"MCP Scraper Report: {self.stats['successful_novels']}/{self.stats['total_novels']} novels processed"
                self._send_notification_email(subject, self._format_stats_report())
            
            # Write report to file
            report_path = SCRIPT_DIR / f"scraper_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.txt"
            with open(report_path, 'w') as f:
                f.write(self._format_stats_report())
            logger.info(f"Report written to {report_path}")
            
        except Exception as e:
            logger.error(f"Error in scraper automation: {str(e)}")
            self.stats["end_time"] = datetime.now()
            
            # Send error notification
            if config.ENABLE_EMAIL_NOTIFICATIONS:
                subject = "MCP Scraper Error"
                body = f"Error in scraper automation: {str(e)}\n\n{self._format_stats_report()}"
                self._send_notification_email(subject, body)
    
    def run_scheduled(self) -> None:
        """Run the scraper on a schedule."""
        if self.run_once:
            logger.info("Running once and exiting")
            self.run()
            return
        
        logger.info(f"Starting scheduled scraping every {config.SCHEDULE_INTERVAL} hours")
        
        try:
            while True:
                # Run the scraper
                self.run()
                
                # Calculate next run time
                next_run = datetime.now() + timedelta(hours=config.SCHEDULE_INTERVAL)
                logger.info(f"Next scraping run scheduled for: {next_run.strftime('%Y-%m-%d %H:%M:%S')}")
                
                # Sleep until next run
                sleep_seconds = (next_run - datetime.now()).total_seconds()
                
                # Check every hour if we should exit
                while sleep_seconds > 0:
                    time.sleep(min(3600, sleep_seconds))
                    sleep_seconds -= 3600
        
        except KeyboardInterrupt:
            logger.info("Scraper automation stopped by user")

def main():
    """Main entry point for the script."""
    parser = argparse.ArgumentParser(description="Automate novel scraping for MCP MongoDB server")
    parser.add_argument("--once", action="store_true", 
                       help="Run once and exit (don't run on schedule)")
    parser.add_argument("--scraper", 
                       help="Run only the specified scraper")
    parser.add_argument("--verbose", action="store_true",
                       help="Enable verbose logging")
    args = parser.parse_args()
    
    # Create and run the automation
    automation = ScraperAutomation(
        run_once=args.once,
        specific_scraper=args.scraper,
        verbose=args.verbose
    )
    
    automation.run_scheduled()

if __name__ == "__main__":
    main()