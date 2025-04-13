#!/bin/bash
# ------------------------------------------------------------------------------
# MCP Database Scraper Automation Script
# ------------------------------------------------------------------------------
# This script automates the scraping of novels using the MCP MongoDB scraper.
# It is optimized for token efficiency and small context windows (3k tokens).
#
# Usage:
#   ./run_scrapers.sh [OPTIONS]
#
# Options:
#   -o, --once       Run once and exit (don't run on schedule)
#   -s, --scraper    Run only the specified scraper
#   -v, --verbose    Enable verbose logging
#   -h, --help       Display this help message
#
# Examples:
#   ./run_scrapers.sh                  # Run all scrapers on schedule
#   ./run_scrapers.sh --once           # Run all scrapers once
#   ./run_scrapers.sh -s 69shu         # Run only the 69shu scraper on schedule
#   ./run_scrapers.sh -o -s syosetu    # Run only the syosetu scraper once
# ------------------------------------------------------------------------------

# Determine the script directory path
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

# Add the project directory to PYTHONPATH
export PYTHONPATH=$PROJECT_DIR:$PYTHONPATH

# Path to Python - use virtual environment if available
if [ -f "$PROJECT_DIR/venv-scraper/bin/python" ]; then
    PYTHON="$PROJECT_DIR/venv-scraper/bin/python"
else
    PYTHON="python3"
fi

# Default arguments
RUN_ONCE=""
SCRAPER=""
VERBOSE=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--once)
            RUN_ONCE="--once"
            shift
            ;;
        -s|--scraper)
            SCRAPER="--scraper $2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE="--verbose"
            shift
            ;;
        -h|--help)
            echo "Usage: ./run_scrapers.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -o, --once       Run once and exit (don't run on schedule)"
            echo "  -s, --scraper    Run only the specified scraper"
            echo "  -v, --verbose    Enable verbose logging"
            echo "  -h, --help       Display this help message"
            echo ""
            echo "Examples:"
            echo "  ./run_scrapers.sh                  # Run all scrapers on schedule"
            echo "  ./run_scrapers.sh --once           # Run all scrapers once"
            echo "  ./run_scrapers.sh -s 69shu         # Run only the 69shu scraper on schedule"
            echo "  ./run_scrapers.sh -o -s syosetu    # Run only the syosetu scraper once"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help to see available options"
            exit 1
            ;;
    esac
done

# Check if MCP MongoDB server is running
echo "Checking if MCP MongoDB server is running..."
if ! curl -s "http://localhost:3000/api/novels" > /dev/null; then
    echo "Warning: MCP MongoDB server does not appear to be running at http://localhost:3000"
    echo "You may need to start the server before running the scrapers."
    echo "Continue anyway? (y/n)"
    read -r CONTINUE
    if [[ ! "$CONTINUE" =~ ^[Yy]$ ]]; then
        echo "Exiting."
        exit 1
    fi
fi

# Create a log directory if it doesn't exist
LOG_DIR="$PROJECT_DIR/logs"
mkdir -p "$LOG_DIR"

# Set up logging with timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$LOG_DIR/scraper_run_$TIMESTAMP.log"

# Display startup message
echo "========================================================"
echo " MCP MongoDB Novel Scraper Automation"
echo " $(date)"
echo "========================================================"
echo "Running with options: $RUN_ONCE $SCRAPER $VERBOSE"
echo "Logging to: $LOG_FILE"
echo "--------------------------------------------------------"

# Run the Python script with appropriate arguments
(
    cd "$PROJECT_DIR" || exit 1
    echo "Starting scraper automation..."
    "$PYTHON" "$SCRIPT_DIR/run_scrapers.py" $RUN_ONCE $SCRAPER $VERBOSE 2>&1 | tee -a "$LOG_FILE"
)

exit 0