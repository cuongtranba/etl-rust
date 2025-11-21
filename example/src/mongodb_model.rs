use mongodb::bson::DateTime as BsonDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    #[serde(rename = "zipCode")]
    pub zip_code: String,
    pub country: String,
    pub coordinates: Coordinates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Education {
    pub institution: String,
    pub degree: String,
    pub year: i32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub company: String,
    pub position: String,
    pub duration: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub bio: String,
    pub interests: Vec<String>,
    pub skills: Vec<String>,
    pub education: Vec<Education>,
    pub experience: Vec<Experience>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub email: bool,
    pub push: bool,
    pub sms: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub timestamp: BsonDateTime,
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub language: String,
    pub timezone: String,
    pub notifications: NotificationSettings,
    pub settings: Vec<Setting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
    pub key: String,
    pub value: String,
    pub timestamp: BsonDateTime,
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub size: i32,
    #[serde(rename = "type")]
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub timestamp: BsonDateTime,
    pub read: bool,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub joined: BsonDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMedia {
    pub posts: Vec<DataEntry>,
    pub connections: Vec<String>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeData {
    pub blob1: String,
    pub blob2: String,
    pub blob3: String,
    pub blob4: String,
    pub blob5: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub age: i32,
    #[serde(rename = "createdAt")]
    pub created_at: BsonDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: BsonDateTime,
    pub address: Address,
    pub profile: Profile,
    pub preferences: Preferences,
    #[serde(rename = "activityLog")]
    pub activity_log: Vec<DataEntry>,
    pub transactions: Vec<DataEntry>,
    pub messages: Vec<Message>,
    #[serde(rename = "socialMedia")]
    pub social_media: SocialMedia,
    #[serde(rename = "largeData")]
    pub large_data: LargeData,
}
