// MongoDB script to create sample database with 10GB of user data
// Run with: docker exec -i mongodb-dev mongosh -u admin -p password123 --authenticationDatabase admin < setup_mongodb_sample.js

// Switch to sample database
use sample;

print("Starting to generate 10GB of user data...");

// Drop existing collection if it exists
db.user.drop();

// Create the user collection
db.createCollection("user");

// Function to generate random user data
function generateUser(id) {
    // Generate random string data to increase document size
    const generateRandomString = (length) => {
        const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
        let result = '';
        for (let i = 0; i < length; i++) {
            result += chars.charAt(Math.floor(Math.random() * chars.length));
        }
        return result;
    };

    // Generate random data arrays to increase size
    const generateDataArray = (size) => {
        const arr = [];
        for (let i = 0; i < size; i++) {
            arr.push({
                key: generateRandomString(20),
                value: generateRandomString(100),
                timestamp: new Date(Date.now() - Math.floor(Math.random() * 365 * 24 * 60 * 60 * 1000)),
                metadata: generateRandomString(50)
            });
        }
        return arr;
    };

    return {
        _id: id,
        username: `user_${id}`,
        email: `user${id}@example.com`,
        firstName: generateRandomString(15),
        lastName: generateRandomString(20),
        age: Math.floor(Math.random() * 60) + 18,
        createdAt: new Date(Date.now() - Math.floor(Math.random() * 365 * 24 * 60 * 60 * 1000)),
        updatedAt: new Date(),
        address: {
            street: generateRandomString(30),
            city: generateRandomString(20),
            state: generateRandomString(15),
            zipCode: generateRandomString(10),
            country: generateRandomString(20),
            coordinates: {
                lat: Math.random() * 180 - 90,
                lng: Math.random() * 360 - 180
            }
        },
        profile: {
            bio: generateRandomString(500),
            interests: Array(20).fill(null).map(() => generateRandomString(25)),
            skills: Array(15).fill(null).map(() => generateRandomString(20)),
            education: Array(5).fill(null).map(() => ({
                institution: generateRandomString(40),
                degree: generateRandomString(30),
                year: 1990 + Math.floor(Math.random() * 30),
                description: generateRandomString(200)
            })),
            experience: Array(8).fill(null).map(() => ({
                company: generateRandomString(35),
                position: generateRandomString(30),
                duration: generateRandomString(20),
                description: generateRandomString(300)
            }))
        },
        preferences: {
            language: generateRandomString(10),
            timezone: generateRandomString(20),
            notifications: {
                email: Math.random() > 0.5,
                push: Math.random() > 0.5,
                sms: Math.random() > 0.5
            },
            settings: generateDataArray(10)
        },
        activityLog: generateDataArray(30),
        transactions: generateDataArray(25),
        messages: Array(50).fill(null).map(() => ({
            id: generateRandomString(24),
            from: generateRandomString(20),
            to: generateRandomString(20),
            subject: generateRandomString(50),
            body: generateRandomString(500),
            timestamp: new Date(Date.now() - Math.floor(Math.random() * 30 * 24 * 60 * 60 * 1000)),
            read: Math.random() > 0.5,
            attachments: Array(3).fill(null).map(() => ({
                name: generateRandomString(30),
                size: Math.floor(Math.random() * 1000000),
                type: generateRandomString(15)
            }))
        })),
        socialMedia: {
            posts: generateDataArray(40),
            connections: Array(100).fill(null).map(() => generateRandomString(24)),
            groups: Array(20).fill(null).map(() => ({
                id: generateRandomString(24),
                name: generateRandomString(30),
                joined: new Date(Date.now() - Math.floor(Math.random() * 365 * 24 * 60 * 60 * 1000))
            }))
        },
        largeData: {
            blob1: generateRandomString(5000),
            blob2: generateRandomString(5000),
            blob3: generateRandomString(5000),
            blob4: generateRandomString(5000),
            blob5: generateRandomString(5000)
        }
    };
}

// Batch insert for better performance
const batchSize = 1000;
let totalInserted = 0;
let totalSize = 0;
const targetSize = 10 * 1024 * 1024 * 1024; // 10GB in bytes

print(`Target size: ${(targetSize / 1024 / 1024 / 1024).toFixed(2)} GB`);

while (totalSize < targetSize) {
    const batch = [];
    const startId = totalInserted + 1;

    // Generate a batch of documents
    for (let i = 0; i < batchSize; i++) {
        batch.push(generateUser(startId + i));
    }

    // Insert the batch
    try {
        db.user.insertMany(batch, { ordered: false });
        totalInserted += batchSize;

        // Check current collection size
        const stats = db.user.stats();
        totalSize = stats.size;

        const progress = (totalSize / targetSize * 100).toFixed(2);
        const sizeGB = (totalSize / 1024 / 1024 / 1024).toFixed(3);

        print(`Progress: ${progress}% | Documents: ${totalInserted} | Size: ${sizeGB} GB`);

        // Add a small delay to avoid overwhelming the system
        sleep(100);
    } catch (e) {
        print(`Error inserting batch: ${e}`);
        break;
    }
}

// Create indexes for better query performance
print("\nCreating indexes...");
db.user.createIndex({ username: 1 });
db.user.createIndex({ email: 1 });
db.user.createIndex({ createdAt: -1 });
db.user.createIndex({ "address.city": 1 });
db.user.createIndex({ age: 1 });

// Final statistics
const finalStats = db.user.stats();
print("\n=== Final Statistics ===");
print(`Total documents: ${db.user.countDocuments()}`);
print(`Collection size: ${(finalStats.size / 1024 / 1024 / 1024).toFixed(3)} GB`);
print(`Storage size: ${(finalStats.storageSize / 1024 / 1024 / 1024).toFixed(3)} GB`);
print(`Average document size: ${(finalStats.avgObjSize / 1024).toFixed(2)} KB`);
print(`Indexes: ${finalStats.nindexes}`);
print("\nDatabase setup complete!");
