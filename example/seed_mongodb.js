// MongoDB sample data seeding script
db = db.getSiblingDB('sample_db');

// Clear existing data
db.users.deleteMany({});

// Insert sample users
db.users.insertMany([
    {
        _id: NumberLong(1),
        username: "john_doe",
        email: "john@example.com",
        firstName: "John",
        lastName: "Doe",
        age: 30,
        createdAt: new Date(),
        updatedAt: new Date(),
        address: {
            street: "123 Main St",
            city: "New York",
            state: "NY",
            zipCode: "10001",
            country: "USA",
            coordinates: {
                lat: 40.7128,
                lng: -74.0060
            }
        },
        profile: {
            bio: "Software engineer passionate about coding",
            interests: ["coding", "reading", "hiking"],
            skills: ["JavaScript", "Python", "Rust"],
            education: [
                {
                    institution: "MIT",
                    degree: "BS Computer Science",
                    year: 2015,
                    description: "Focused on algorithms and data structures"
                }
            ],
            experience: [
                {
                    company: "Tech Corp",
                    position: "Senior Engineer",
                    duration: "2015-Present",
                    description: "Leading backend development"
                }
            ]
        },
        preferences: {
            language: "en",
            timezone: "America/New_York",
            notifications: {
                email: true,
                push: true,
                sms: false
            },
            settings: [
                {
                    key: "theme",
                    value: "dark",
                    timestamp: new Date(),
                    metadata: "{}"
                }
            ]
        },
        activityLog: [
            {
                key: "login",
                value: "success",
                timestamp: new Date(),
                metadata: "{\"ip\": \"192.168.1.1\"}"
            }
        ],
        transactions: [
            {
                key: "purchase",
                value: "100",
                timestamp: new Date(),
                metadata: "{\"item\": \"book\"}"
            }
        ],
        messages: [
            {
                id: "msg1",
                from: "admin@example.com",
                to: "john@example.com",
                subject: "Welcome",
                body: "Welcome to our platform!",
                timestamp: new Date(),
                read: false,
                attachments: []
            }
        ],
        socialMedia: {
            connections: ["user2", "user3"],
            posts: [
                {
                    key: "post1",
                    value: "Hello world!",
                    timestamp: new Date(),
                    metadata: "{\"likes\": 10}"
                }
            ],
            groups: [
                {
                    id: "group1",
                    name: "Developers",
                    joined: new Date()
                }
            ]
        },
        largeData: {
            blob1: "data1".repeat(10),
            blob2: "data2".repeat(10),
            blob3: "data3".repeat(10),
            blob4: "data4".repeat(10),
            blob5: "data5".repeat(10)
        }
    },
    {
        _id: NumberLong(2),
        username: "jane_smith",
        email: "jane@example.com",
        firstName: "Jane",
        lastName: "Smith",
        age: 28,
        createdAt: new Date(),
        updatedAt: new Date(),
        address: {
            street: "456 Oak Ave",
            city: "San Francisco",
            state: "CA",
            zipCode: "94102",
            country: "USA",
            coordinates: {
                lat: 37.7749,
                lng: -122.4194
            }
        },
        profile: {
            bio: "Product manager and tech enthusiast",
            interests: ["technology", "design", "travel"],
            skills: ["Product Management", "UX Design", "Agile"],
            education: [
                {
                    institution: "Stanford",
                    degree: "MBA",
                    year: 2018,
                    description: "Technology and entrepreneurship focus"
                }
            ],
            experience: [
                {
                    company: "Startup Inc",
                    position: "Product Manager",
                    duration: "2018-Present",
                    description: "Managing product roadmap"
                }
            ]
        },
        preferences: {
            language: "en",
            timezone: "America/Los_Angeles",
            notifications: {
                email: true,
                push: false,
                sms: true
            },
            settings: [
                {
                    key: "theme",
                    value: "light",
                    timestamp: new Date(),
                    metadata: "{}"
                }
            ]
        },
        activityLog: [
            {
                key: "login",
                value: "success",
                timestamp: new Date(),
                metadata: "{\"ip\": \"192.168.1.2\"}"
            }
        ],
        transactions: [],
        messages: [],
        socialMedia: {
            connections: ["user1"],
            posts: [],
            groups: []
        },
        largeData: {
            blob1: "blob1".repeat(10),
            blob2: "blob2".repeat(10),
            blob3: "blob3".repeat(10),
            blob4: "blob4".repeat(10),
            blob5: "blob5".repeat(10)
        }
    }
]);

print("Seeded " + db.users.countDocuments() + " users successfully!");
