use sea_orm::entity::prelude::*;

pub mod users {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub username: String,
        pub email: String,
        pub first_name: String,
        pub last_name: String,
        pub age: i32,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_one = "super::addresses::Entity")]
        Address,
        #[sea_orm(has_one = "super::profiles::Entity")]
        Profile,
        #[sea_orm(has_one = "super::preferences::Entity")]
        Preferences,
        #[sea_orm(has_many = "super::activity_log::Entity")]
        ActivityLog,
        #[sea_orm(has_many = "super::transactions::Entity")]
        Transactions,
        #[sea_orm(has_many = "super::messages::Entity")]
        Messages,
        #[sea_orm(has_one = "super::social_media::Entity")]
        SocialMedia,
        #[sea_orm(has_one = "super::large_data::Entity")]
        LargeData,
    }

    impl Related<super::addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Address.def()
        }
    }

    impl Related<super::profiles::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Profile.def()
        }
    }

    impl Related<super::preferences::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Preferences.def()
        }
    }

    impl Related<super::activity_log::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ActivityLog.def()
        }
    }

    impl Related<super::transactions::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Transactions.def()
        }
    }

    impl Related<super::messages::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Messages.def()
        }
    }

    impl Related<super::social_media::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::SocialMedia.def()
        }
    }

    impl Related<super::large_data::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::LargeData.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod addresses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "addresses")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub user_id: i64,
        pub street: String,
        pub city: String,
        pub state: String,
        pub zip_code: String,
        pub country: String,
        pub coordinates: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod profiles {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "profiles")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub user_id: i64,
        pub bio: String,
        pub interests: Json,
        pub skills: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
        #[sea_orm(has_many = "super::education::Entity")]
        Education,
        #[sea_orm(has_many = "super::experience::Entity")]
        Experience,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::education::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Education.def()
        }
    }

    impl Related<super::experience::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Experience.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod education {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "education")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub profile_id: i64,
        pub institution: String,
        pub degree: String,
        pub year: i32,
        pub description: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::profiles::Entity",
            from = "Column::ProfileId",
            to = "super::profiles::Column::Id"
        )]
        Profile,
    }

    impl Related<super::profiles::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Profile.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod experience {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "experience")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub profile_id: i64,
        pub company: String,
        pub position: String,
        pub duration: String,
        pub description: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::profiles::Entity",
            from = "Column::ProfileId",
            to = "super::profiles::Column::Id"
        )]
        Profile,
    }

    impl Related<super::profiles::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Profile.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod preferences {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "preferences")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub user_id: i64,
        pub language: String,
        pub timezone: String,
        pub notifications: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
        #[sea_orm(has_many = "super::settings::Entity")]
        Settings,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::settings::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Settings.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod settings {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "settings")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub preference_id: i64,
        pub key: String,
        pub value: String,
        pub timestamp: DateTimeWithTimeZone,
        pub metadata: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::preferences::Entity",
            from = "Column::PreferenceId",
            to = "super::preferences::Column::Id"
        )]
        Preference,
    }

    impl Related<super::preferences::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Preference.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod activity_log {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "activity_log")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub user_id: i64,
        pub key: String,
        pub value: String,
        pub timestamp: DateTimeWithTimeZone,
        pub metadata: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod transactions {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "transactions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub user_id: i64,
        pub key: String,
        pub value: String,
        pub timestamp: DateTimeWithTimeZone,
        pub metadata: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod messages {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "messages")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: String,
        pub user_id: i64,
        pub from: String,
        pub to: String,
        pub subject: String,
        pub body: String,
        pub timestamp: DateTimeWithTimeZone,
        pub read: bool,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
        #[sea_orm(has_many = "super::attachments::Entity")]
        Attachments,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::attachments::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Attachments.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod attachments {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "attachments")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub message_id: String,
        pub name: String,
        pub size: i32,
        pub file_type: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::messages::Entity",
            from = "Column::MessageId",
            to = "super::messages::Column::Id"
        )]
        Message,
    }

    impl Related<super::messages::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Message.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod social_media {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "social_media")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub user_id: i64,
        pub connections: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
        #[sea_orm(has_many = "super::posts::Entity")]
        Posts,
        #[sea_orm(has_many = "super::groups::Entity")]
        Groups,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::posts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Posts.def()
        }
    }

    impl Related<super::groups::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Groups.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod posts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub social_media_id: i64,
        pub key: String,
        pub value: String,
        pub timestamp: DateTimeWithTimeZone,
        pub metadata: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::social_media::Entity",
            from = "Column::SocialMediaId",
            to = "super::social_media::Column::Id"
        )]
        SocialMedia,
    }

    impl Related<super::social_media::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::SocialMedia.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod groups {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "groups")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: String,
        pub social_media_id: i64,
        pub name: String,
        pub joined: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::social_media::Entity",
            from = "Column::SocialMediaId",
            to = "super::social_media::Column::Id"
        )]
        SocialMedia,
    }

    impl Related<super::social_media::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::SocialMedia.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod large_data {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "large_data")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub user_id: i64,
        pub blob1: String,
        pub blob2: String,
        pub blob3: String,
        pub blob4: String,
        pub blob5: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
