#!/usr/bin/env python3
"""
MongoDB Sample Database Setup Script
Creates a sample database with 10GB of user data
"""

import random
import string
from datetime import datetime, timedelta
from pymongo import MongoClient
import time

# MongoDB connection
MONGO_URI = "mongodb://admin:password123@localhost:27017/"
DATABASE_NAME = "sample"
COLLECTION_NAME = "user"
TARGET_SIZE_GB = 10
BATCH_SIZE = 500

def random_string(length):
    """Generate a random string of specified length"""
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))

def random_date(start_days_ago=365):
    """Generate a random date within the last N days"""
    days_ago = random.randint(0, start_days_ago)
    return datetime.now() - timedelta(days=days_ago)

def generate_data_array(size):
    """Generate an array of data objects"""
    return [
        {
            "key": random_string(20),
            "value": random_string(100),
            "timestamp": random_date(),
            "metadata": random_string(50)
        }
        for _ in range(size)
    ]

def generate_user(user_id):
    """Generate a single user document with lots of data"""
    return {
        "_id": user_id,
        "username": f"user_{user_id}",
        "email": f"user{user_id}@example.com",
        "firstName": random_string(15),
        "lastName": random_string(20),
        "age": random.randint(18, 78),
        "createdAt": random_date(365),
        "updatedAt": datetime.now(),
        "address": {
            "street": random_string(30),
            "city": random_string(20),
            "state": random_string(15),
            "zipCode": random_string(10),
            "country": random_string(20),
            "coordinates": {
                "lat": random.uniform(-90, 90),
                "lng": random.uniform(-180, 180)
            }
        },
        "profile": {
            "bio": random_string(500),
            "interests": [random_string(25) for _ in range(20)],
            "skills": [random_string(20) for _ in range(15)],
            "education": [
                {
                    "institution": random_string(40),
                    "degree": random_string(30),
                    "year": random.randint(1990, 2024),
                    "description": random_string(200)
                }
                for _ in range(5)
            ],
            "experience": [
                {
                    "company": random_string(35),
                    "position": random_string(30),
                    "duration": random_string(20),
                    "description": random_string(300)
                }
                for _ in range(8)
            ]
        },
        "preferences": {
            "language": random_string(10),
            "timezone": random_string(20),
            "notifications": {
                "email": random.choice([True, False]),
                "push": random.choice([True, False]),
                "sms": random.choice([True, False])
            },
            "settings": generate_data_array(10)
        },
        "activityLog": generate_data_array(30),
        "transactions": generate_data_array(25),
        "messages": [
            {
                "id": random_string(24),
                "from": random_string(20),
                "to": random_string(20),
                "subject": random_string(50),
                "body": random_string(500),
                "timestamp": random_date(30),
                "read": random.choice([True, False]),
                "attachments": [
                    {
                        "name": random_string(30),
                        "size": random.randint(1000, 1000000),
                        "type": random_string(15)
                    }
                    for _ in range(3)
                ]
            }
            for _ in range(50)
        ],
        "socialMedia": {
            "posts": generate_data_array(40),
            "connections": [random_string(24) for _ in range(100)],
            "groups": [
                {
                    "id": random_string(24),
                    "name": random_string(30),
                    "joined": random_date(365)
                }
                for _ in range(20)
            ]
        },
        "largeData": {
            "blob1": random_string(5000),
            "blob2": random_string(5000),
            "blob3": random_string(5000),
            "blob4": random_string(5000),
            "blob5": random_string(5000)
        }
    }

def get_collection_size(collection):
    """Get the current size of the collection in bytes"""
    stats = collection.database.command("collStats", collection.name)
    return stats.get("size", 0)

def main():
    print("=" * 50)
    print("MongoDB Sample Database Setup")
    print("=" * 50)
    print(f"\nTarget size: {TARGET_SIZE_GB} GB")
    print(f"Database: {DATABASE_NAME}")
    print(f"Collection: {COLLECTION_NAME}")
    print(f"Batch size: {BATCH_SIZE} documents")
    print("\nConnecting to MongoDB...")
    
    try:
        # Connect to MongoDB
        client = MongoClient(MONGO_URI, serverSelectionTimeoutMS=5000)
        client.server_info()
        print("✓ Connected to MongoDB successfully")
        
        # Get database and collection
        db = client[DATABASE_NAME]
        collection = db[COLLECTION_NAME]
        
        # Drop existing collection
        print(f"\nDropping existing '{COLLECTION_NAME}' collection if it exists...")
        collection.drop()
        print("✓ Collection dropped")
        
        # Generate and insert data
        target_size_bytes = TARGET_SIZE_GB * 1024 * 1024 * 1024
        total_inserted = 0
        start_time = time.time()
        
        print(f"\nStarting data generation (target: {TARGET_SIZE_GB} GB)...")
        print("This will take approximately 10-15 minutes...\n")
        
        while True:
            # Check current size
            current_size = get_collection_size(collection)
            
            if current_size >= target_size_bytes:
                break
            
            # Generate batch of documents
            batch = []
            for i in range(BATCH_SIZE):
                user_id = total_inserted + i + 1
                batch.append(generate_user(user_id))
            
            # Insert batch
            try:
                collection.insert_many(batch, ordered=False)
                total_inserted += BATCH_SIZE
                
                # Progress update
                progress = (current_size / target_size_bytes) * 100
                size_gb = current_size / (1024 * 1024 * 1024)
                elapsed = time.time() - start_time
                docs_per_sec = total_inserted / elapsed if elapsed > 0 else 0
                
                print(f"Progress: {progress:5.2f}% | "
                      f"Documents: {total_inserted:,} | "
                      f"Size: {size_gb:.3f} GB | "
                      f"Speed: {docs_per_sec:.0f} docs/sec")
                
            except Exception as e:
                print(f"Error inserting batch: {e}")
                continue
        
        # Create indexes
        print("\n\nCreating indexes...")
        collection.create_index("username")
        collection.create_index("email")
        collection.create_index("createdAt")
        collection.create_index("address.city")
        collection.create_index("age")
        print("✓ Indexes created")
        
        # Final statistics
        final_size = get_collection_size(collection)
        doc_count = collection.count_documents({})
        stats = db.command("collStats", COLLECTION_NAME)
        
        elapsed_time = time.time() - start_time
        
        print("\n" + "=" * 50)
        print("Setup Complete!")
        print("=" * 50)
        print(f"Total documents:       {doc_count:,}")
        print(f"Collection size:       {final_size / (1024**3):.3f} GB")
        print(f"Storage size:          {stats.get('storageSize', 0) / (1024**3):.3f} GB")
        print(f"Average document size: {stats.get('avgObjSize', 0) / 1024:.2f} KB")
        print(f"Number of indexes:     {stats.get('nindexes', 0)}")
        print(f"Time elapsed:          {elapsed_time / 60:.2f} minutes")
        print(f"\nConnection string: {MONGO_URI}{DATABASE_NAME}")
        
    except Exception as e:
        print(f"\n✗ Error: {e}")
        return 1
    
    finally:
        if 'client' in locals():
            client.close()
    
    return 0

if __name__ == "__main__":
    exit(main())
