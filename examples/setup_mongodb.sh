#!/bin/bash

# MongoDB Sample Database Setup Script
# This script creates a sample database with 10GB of user data

echo "======================================"
echo "MongoDB Sample Database Setup"
echo "======================================"

# Check if MongoDB container is running
if ! docker ps | grep -q mongodb-dev; then
    echo "Error: MongoDB container 'mongodb-dev' is not running."
    echo "Please start it with: docker start mongodb-dev"
    exit 1
fi

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is not installed."
    echo "Please install Python 3 to run this script."
    exit 1
fi

# Check if pymongo is installed, if not install it
if ! python3 -c "import pymongo" &> /dev/null; then
    echo "Installing pymongo..."
    python3 -m pip install --break-system-packages pymongo 2>/dev/null || \
    python3 -m pip install --user pymongo 2>/dev/null || \
    python3 -m pip install pymongo
    
    if ! python3 -c "import pymongo" &> /dev/null; then
        echo "Error: Failed to install pymongo."
        echo "Please install it manually with one of these commands:"
        echo "  pip3 install --break-system-packages pymongo"
        echo "  pip3 install --user pymongo"
        exit 1
    fi
    echo "✓ pymongo installed successfully"
fi

echo ""
echo "This script will create:"
echo "- Database: sample"
echo "- Collection: user"
echo "- Data size: ~10GB of dummy user records"
echo ""
echo "WARNING: This will take several minutes to complete!"
echo "Press Ctrl+C to cancel, or Enter to continue..."
read

# Execute the Python script
echo ""

python3 setup_mongodb_sample.py

if [ $? -eq 0 ]; then
    echo ""
    echo "To verify the data, run:"
    echo "  docker exec -it mongodb-dev mongosh -u admin -p password123 --authenticationDatabase admin"
    echo "  Then: use sample"
    echo "        db.user.countDocuments()"
    echo "        db.user.stats()"
else
    echo ""
    echo "Error: Setup failed. Please check the error messages above."
    exit 1
fi
