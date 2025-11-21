#!/usr/bin/env python3
"""
Generate large dataset for MongoDB ETL testing
"""

import random
import string
from datetime import datetime, timedelta

from pymongo import MongoClient


def random_string(length=10):
    return "".join(random.choices(string.ascii_letters, k=length))


def random_email():
    return f"{random_string(8).lower()}@example.com"


def random_date():
    days_ago = random.randint(1, 365 * 5)
    return datetime.utcnow() - timedelta(days=days_ago)


def generate_user(user_id):
    """Generate a single user document"""
    first_names = [
        "John",
        "Jane",
        "Michael",
        "Sarah",
        "David",
        "Emily",
        "Robert",
        "Lisa",
        "James",
        "Maria",
        "William",
        "Jennifer",
        "Richard",
        "Patricia",
        "Thomas",
    ]
    last_names = [
        "Smith",
        "Johnson",
        "Williams",
        "Brown",
        "Jones",
        "Garcia",
        "Miller",
        "Davis",
        "Rodriguez",
        "Martinez",
        "Hernandez",
        "Lopez",
        "Wilson",
        "Anderson",
    ]
    cities = [
        "New York",
        "Los Angeles",
        "Chicago",
        "Houston",
        "Phoenix",
        "Philadelphia",
        "San Antonio",
        "San Diego",
        "Dallas",
        "San Jose",
    ]
    states = ["NY", "CA", "IL", "TX", "AZ", "PA", "FL", "OH", "NC", "GA"]

    first_name = random.choice(first_names)
    last_name = random.choice(last_names)
    username = f"{first_name.lower()}_{last_name.lower()}_{user_id}"

    num_education = random.randint(1, 3)
    num_experience = random.randint(1, 4)
    num_settings = random.randint(1, 5)
    num_activity_logs = random.randint(5, 20)
    num_transactions = random.randint(0, 10)
    num_messages = random.randint(0, 5)
    num_posts = random.randint(0, 10)
    num_groups = random.randint(0, 3)

    user = {
        "_id": user_id,
        "username": username,
        "email": f"{username}@example.com",
        "firstName": first_name,
        "lastName": last_name,
        "age": random.randint(18, 80),
        "createdAt": random_date(),
        "updatedAt": datetime.utcnow(),
        "address": {
            "street": f"{random.randint(100, 9999)} {random.choice(['Main', 'Oak', 'Maple', 'Pine'])} {random.choice(['St', 'Ave', 'Blvd', 'Dr'])}",
            "city": random.choice(cities),
            "state": random.choice(states),
            "zipCode": f"{random.randint(10000, 99999)}",
            "country": "USA",
            "coordinates": {
                "lat": random.uniform(25.0, 48.0),
                "lng": random.uniform(-125.0, -65.0),
            },
        },
        "profile": {
            "bio": f"Professional {random.choice(['engineer', 'designer', 'manager', 'analyst', 'developer'])} with {random.randint(1, 20)} years of experience",
            "interests": random.sample(
                [
                    "coding",
                    "reading",
                    "hiking",
                    "gaming",
                    "music",
                    "travel",
                    "sports",
                    "photography",
                ],
                k=random.randint(2, 5),
            ),
            "skills": random.sample(
                [
                    "JavaScript",
                    "Python",
                    "Rust",
                    "Go",
                    "Java",
                    "C++",
                    "React",
                    "Node.js",
                    "SQL",
                    "AWS",
                ],
                k=random.randint(2, 6),
            ),
            "education": [
                {
                    "institution": random.choice(
                        [
                            "MIT",
                            "Stanford",
                            "Harvard",
                            "UC Berkeley",
                            "Carnegie Mellon",
                            "State University",
                        ]
                    ),
                    "degree": random.choice(
                        [
                            "BS Computer Science",
                            "MS Engineering",
                            "MBA",
                            "PhD",
                            "BA Business",
                        ]
                    ),
                    "year": random.randint(1990, 2023),
                    "description": random_string(50),
                }
                for _ in range(num_education)
            ],
            "experience": [
                {
                    "company": f"{random.choice(['Tech', 'Data', 'Cloud', 'Cyber', 'Digital'])} {random.choice(['Corp', 'Inc', 'Systems', 'Solutions'])}",
                    "position": random.choice(
                        [
                            "Software Engineer",
                            "Senior Developer",
                            "Tech Lead",
                            "Manager",
                            "Director",
                        ]
                    ),
                    "duration": f"{random.randint(2015, 2020)}-{random.choice(['Present', '2023', '2022'])}",
                    "description": random_string(50),
                }
                for _ in range(num_experience)
            ],
        },
        "preferences": {
            "language": random.choice(["en", "es", "fr", "de"]),
            "timezone": random.choice(
                [
                    "America/New_York",
                    "America/Los_Angeles",
                    "America/Chicago",
                    "Europe/London",
                ]
            ),
            "notifications": {
                "email": random.choice([True, False]),
                "push": random.choice([True, False]),
                "sms": random.choice([True, False]),
            },
            "settings": [
                {
                    "key": f"setting_{i}",
                    "value": random_string(10),
                    "timestamp": random_date(),
                    "metadata": "{}",
                }
                for i in range(num_settings)
            ],
        },
        "activityLog": [
            {
                "key": random.choice(["login", "logout", "update", "view", "delete"]),
                "value": random.choice(["success", "failure", "pending"]),
                "timestamp": random_date(),
                "metadata": f'{{"ip": "192.168.{random.randint(1, 255)}.{random.randint(1, 255)}"}}',
            }
            for _ in range(num_activity_logs)
        ],
        "transactions": [
            {
                "key": random.choice(["purchase", "refund", "payment", "transfer"]),
                "value": str(random.randint(10, 1000)),
                "timestamp": random_date(),
                "metadata": f'{{"amount": {random.randint(10, 1000)}}}',
            }
            for _ in range(num_transactions)
        ],
        "messages": [
            {
                "id": f"msg{user_id}_{i}",
                "from": random_email(),
                "to": f"{username}@example.com",
                "subject": f"Message {i}",
                "body": random_string(100),
                "timestamp": random_date(),
                "read": random.choice([True, False]),
                "attachments": [],
            }
            for i in range(num_messages)
        ],
        "socialMedia": {
            "connections": [
                f"user{random.randint(1, 10000)}" for _ in range(random.randint(0, 20))
            ],
            "posts": [
                {
                    "key": f"post{i}",
                    "value": random_string(50),
                    "timestamp": random_date(),
                    "metadata": f'{{"likes": {random.randint(0, 100)}}}',
                }
                for i in range(num_posts)
            ],
            "groups": [
                {
                    "id": f"group{user_id}_{i}",
                    "name": f"{random.choice(['Tech', 'Dev', 'Design', 'Data'])} {random.choice(['Group', 'Community', 'Club'])}",
                    "joined": random_date(),
                }
                for i in range(num_groups)
            ],
        },
        "largeData": {
            "blob1": random_string(100),
            "blob2": random_string(100),
            "blob3": random_string(100),
            "blob4": random_string(100),
            "blob5": random_string(100),
        },
    }

    return user


def main():
    import time

    # Connect to MongoDB
    print("Connecting to MongoDB...")
    client = MongoClient("mongodb://admin:password123@localhost:27017/")
    db = client["sample_db"]
    collection = db["users"]
    print("✓ Connected successfully!\n")

    # Generate 50 million records
    num_users = 10_000_000

    print(f"=" * 70)
    print(f"Generating {num_users:,} users (50 Million Records)")
    print(f"=" * 70)
    print("⚠️  This will take significant time and disk space!")
    print(f"   Estimated disk space: ~150-200 GB")
    print(f"   Estimated time: 10-15 hours (depending on hardware)")
    print()

    # Ask for confirmation
    confirm = input("Do you want to proceed? (yes/no): ").lower()
    if confirm not in ["yes", "y"]:
        print("Aborted.")
        return

    # Clear existing data
    print("\nClearing existing data...")
    result = collection.delete_many({})
    print(f"✓ Cleared {result.deleted_count:,} existing documents\n")

    # Optimized batch size for large datasets
    batch_size = 1000  # Larger batches for better performance
    total_inserted = 0
    start_time = time.time()
    last_report_time = start_time

    print("Starting data generation...")
    print(f"Batch size: {batch_size:,} records per batch")
    print(f"Total batches: {num_users // batch_size:,}")
    print("-" * 70)

    for batch_start in range(1, num_users + 1, batch_size):
        batch_end = min(batch_start + batch_size, num_users + 1)
        batch = []

        # Generate batch
        for user_id in range(batch_start, batch_end):
            user = generate_user(user_id)
            batch.append(user)

        # Insert batch
        if batch:
            collection.insert_many(batch, ordered=False)
            total_inserted += len(batch)

            # Progress reporting (every 10,000 records or 30 seconds)
            current_time = time.time()
            if total_inserted % 10_000 == 0 or (current_time - last_report_time) >= 30:
                elapsed = current_time - start_time
                rate = total_inserted / elapsed if elapsed > 0 else 0
                remaining = (num_users - total_inserted) / rate if rate > 0 else 0

                progress_pct = (total_inserted / num_users) * 100
                print(
                    f"Progress: {total_inserted:,}/{num_users:,} ({progress_pct:.2f}%) | "
                    f"Rate: {rate:.0f} docs/sec | "
                    f"Elapsed: {elapsed / 3600:.2f}h | "
                    f"ETA: {remaining / 3600:.2f}h"
                )

                last_report_time = current_time

    # Final statistics
    total_time = time.time() - start_time
    final_count = collection.count_documents({})

    print("\n" + "=" * 70)
    print(f"✓ Successfully generated {final_count:,} users in MongoDB!")
    print("=" * 70)
    print(f"Database: sample_db")
    print(f"Collection: users")
    print(f"Total time: {total_time / 3600:.2f} hours ({total_time / 60:.1f} minutes)")
    print(f"Average rate: {final_count / total_time:.0f} documents/second")
    print()

    # Show sample
    print("Sample user:")
    sample = collection.find_one()
    if sample:
        print(f"  ID: {sample['_id']}")
        print(f"  Username: {sample['username']}")
        print(f"  Email: {sample['email']}")
        print(f"  Education records: {len(sample['profile']['education'])}")
        print(f"  Experience records: {len(sample['profile']['experience'])}")
        print(f"  Activity logs: {len(sample['activityLog'])}")

    # Database size info
    stats = db.command("collstats", "users")
    size_gb = stats.get("size", 0) / (1024**3)
    print(f"\nCollection size: {size_gb:.2f} GB")
    print(f"Storage size: {stats.get('storageSize', 0) / (1024**3):.2f} GB")


if __name__ == "__main__":
    main()
