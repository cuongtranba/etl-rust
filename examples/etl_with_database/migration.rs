use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231121_000001_create_users_table::Migration),
            Box::new(m20231121_000002_create_addresses_table::Migration),
            Box::new(m20231121_000003_create_profiles_table::Migration),
            Box::new(m20231121_000004_create_education_table::Migration),
            Box::new(m20231121_000005_create_experience_table::Migration),
            Box::new(m20231121_000006_create_preferences_table::Migration),
            Box::new(m20231121_000007_create_settings_table::Migration),
            Box::new(m20231121_000008_create_activity_log_table::Migration),
            Box::new(m20231121_000009_create_transactions_table::Migration),
            Box::new(m20231121_000010_create_messages_table::Migration),
            Box::new(m20231121_000011_create_attachments_table::Migration),
            Box::new(m20231121_000012_create_social_media_table::Migration),
            Box::new(m20231121_000013_create_posts_table::Migration),
            Box::new(m20231121_000014_create_groups_table::Migration),
            Box::new(m20231121_000015_create_large_data_table::Migration),
        ]
    }
}

pub mod m20231121_000001_create_users_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Users::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Users::Id)
                                .big_integer()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Users::Username).string().not_null())
                        .col(ColumnDef::new(Users::Email).string().not_null())
                        .col(ColumnDef::new(Users::FirstName).string().not_null())
                        .col(ColumnDef::new(Users::LastName).string().not_null())
                        .col(ColumnDef::new(Users::Age).integer().not_null())
                        .col(
                            ColumnDef::new(Users::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Users::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Users::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Users {
        Table,
        Id,
        Username,
        Email,
        FirstName,
        LastName,
        Age,
        CreatedAt,
        UpdatedAt,
    }
}

pub mod m20231121_000002_create_addresses_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Addresses::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Addresses::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Addresses::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Addresses::Street).string().not_null())
                        .col(ColumnDef::new(Addresses::City).string().not_null())
                        .col(ColumnDef::new(Addresses::State).string().not_null())
                        .col(ColumnDef::new(Addresses::ZipCode).string().not_null())
                        .col(ColumnDef::new(Addresses::Country).string().not_null())
                        .col(ColumnDef::new(Addresses::Coordinates).json().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_addresses_user_id")
                                .from(Addresses::Table, Addresses::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Addresses::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Addresses {
        Table,
        Id,
        UserId,
        Street,
        City,
        State,
        ZipCode,
        Country,
        Coordinates,
    }
}

pub mod m20231121_000003_create_profiles_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Profiles::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Profiles::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Profiles::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Profiles::Bio).string().not_null())
                        .col(ColumnDef::new(Profiles::Interests).json().not_null())
                        .col(ColumnDef::new(Profiles::Skills).json().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_profiles_user_id")
                                .from(Profiles::Table, Profiles::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Profiles::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Profiles {
        Table,
        Id,
        UserId,
        Bio,
        Interests,
        Skills,
    }
}

pub mod m20231121_000004_create_education_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Education::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Education::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(Education::ProfileId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Education::Institution).string().not_null())
                        .col(ColumnDef::new(Education::Degree).string().not_null())
                        .col(ColumnDef::new(Education::Year).integer().not_null())
                        .col(ColumnDef::new(Education::Description).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_education_profile_id")
                                .from(Education::Table, Education::ProfileId)
                                .to(
                                    super::m20231121_000003_create_profiles_table::Profiles::Table,
                                    super::m20231121_000003_create_profiles_table::Profiles::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Education::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Education {
        Table,
        Id,
        ProfileId,
        Institution,
        Degree,
        Year,
        Description,
    }
}

pub mod m20231121_000005_create_experience_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Experience::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Experience::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(Experience::ProfileId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Experience::Company).string().not_null())
                        .col(ColumnDef::new(Experience::Position).string().not_null())
                        .col(ColumnDef::new(Experience::Duration).string().not_null())
                        .col(ColumnDef::new(Experience::Description).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_experience_profile_id")
                                .from(Experience::Table, Experience::ProfileId)
                                .to(
                                    super::m20231121_000003_create_profiles_table::Profiles::Table,
                                    super::m20231121_000003_create_profiles_table::Profiles::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Experience::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Experience {
        Table,
        Id,
        ProfileId,
        Company,
        Position,
        Duration,
        Description,
    }
}

pub mod m20231121_000006_create_preferences_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Preferences::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Preferences::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Preferences::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Preferences::Language).string().not_null())
                        .col(ColumnDef::new(Preferences::Timezone).string().not_null())
                        .col(ColumnDef::new(Preferences::Notifications).json().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_preferences_user_id")
                                .from(Preferences::Table, Preferences::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Preferences::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Preferences {
        Table,
        Id,
        UserId,
        Language,
        Timezone,
        Notifications,
    }
}

pub mod m20231121_000007_create_settings_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Settings::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(Settings::Id).big_integer().not_null().auto_increment().primary_key())
                        .col(ColumnDef::new(Settings::PreferenceId).big_integer().not_null())
                        .col(ColumnDef::new(Settings::Key).string().not_null())
                        .col(ColumnDef::new(Settings::Value).string().not_null())
                        .col(ColumnDef::new(Settings::Timestamp).timestamp_with_time_zone().not_null())
                        .col(ColumnDef::new(Settings::Metadata).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_settings_preference_id")
                                .from(Settings::Table, Settings::PreferenceId)
                                .to(super::m20231121_000006_create_preferences_table::Preferences::Table, super::m20231121_000006_create_preferences_table::Preferences::Id)
                                .on_delete(ForeignKeyAction::Cascade)
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Settings::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Settings {
        Table,
        Id,
        PreferenceId,
        Key,
        Value,
        Timestamp,
        Metadata,
    }
}

pub mod m20231121_000008_create_activity_log_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(ActivityLog::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(ActivityLog::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(ActivityLog::UserId).big_integer().not_null())
                        .col(ColumnDef::new(ActivityLog::Key).string().not_null())
                        .col(ColumnDef::new(ActivityLog::Value).string().not_null())
                        .col(
                            ColumnDef::new(ActivityLog::Timestamp)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(ColumnDef::new(ActivityLog::Metadata).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_activity_log_user_id")
                                .from(ActivityLog::Table, ActivityLog::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(ActivityLog::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum ActivityLog {
        Table,
        Id,
        UserId,
        Key,
        Value,
        Timestamp,
        Metadata,
    }
}

pub mod m20231121_000009_create_transactions_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Transactions::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Transactions::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(Transactions::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Transactions::Key).string().not_null())
                        .col(ColumnDef::new(Transactions::Value).string().not_null())
                        .col(
                            ColumnDef::new(Transactions::Timestamp)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Transactions::Metadata).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_transactions_user_id")
                                .from(Transactions::Table, Transactions::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Transactions::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Transactions {
        Table,
        Id,
        UserId,
        Key,
        Value,
        Timestamp,
        Metadata,
    }
}

pub mod m20231121_000010_create_messages_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Messages::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Messages::Id)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Messages::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Messages::From).string().not_null())
                        .col(ColumnDef::new(Messages::To).string().not_null())
                        .col(ColumnDef::new(Messages::Subject).string().not_null())
                        .col(ColumnDef::new(Messages::Body).string().not_null())
                        .col(
                            ColumnDef::new(Messages::Timestamp)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Messages::Read).boolean().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_messages_user_id")
                                .from(Messages::Table, Messages::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Messages::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Messages {
        Table,
        Id,
        UserId,
        From,
        To,
        Subject,
        Body,
        Timestamp,
        Read,
    }
}

pub mod m20231121_000011_create_attachments_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Attachments::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Attachments::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Attachments::MessageId).string().not_null())
                        .col(ColumnDef::new(Attachments::Name).string().not_null())
                        .col(ColumnDef::new(Attachments::Size).integer().not_null())
                        .col(ColumnDef::new(Attachments::FileType).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_attachments_message_id")
                                .from(Attachments::Table, Attachments::MessageId)
                                .to(
                                    super::m20231121_000010_create_messages_table::Messages::Table,
                                    super::m20231121_000010_create_messages_table::Messages::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Attachments::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Attachments {
        Table,
        Id,
        MessageId,
        Name,
        Size,
        FileType,
    }
}

pub mod m20231121_000012_create_social_media_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(SocialMedia::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(SocialMedia::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(SocialMedia::UserId).big_integer().not_null())
                        .col(ColumnDef::new(SocialMedia::Connections).json().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_social_media_user_id")
                                .from(SocialMedia::Table, SocialMedia::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(SocialMedia::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum SocialMedia {
        Table,
        Id,
        UserId,
        Connections,
    }
}

pub mod m20231121_000013_create_posts_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Posts::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(Posts::Id).big_integer().not_null().auto_increment().primary_key())
                        .col(ColumnDef::new(Posts::SocialMediaId).big_integer().not_null())
                        .col(ColumnDef::new(Posts::Key).string().not_null())
                        .col(ColumnDef::new(Posts::Value).string().not_null())
                        .col(ColumnDef::new(Posts::Timestamp).timestamp_with_time_zone().not_null())
                        .col(ColumnDef::new(Posts::Metadata).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_posts_social_media_id")
                                .from(Posts::Table, Posts::SocialMediaId)
                                .to(super::m20231121_000012_create_social_media_table::SocialMedia::Table, super::m20231121_000012_create_social_media_table::SocialMedia::Id)
                                .on_delete(ForeignKeyAction::Cascade)
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Posts::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Posts {
        Table,
        Id,
        SocialMediaId,
        Key,
        Value,
        Timestamp,
        Metadata,
    }
}

pub mod m20231121_000014_create_groups_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Groups::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(Groups::Id).string().not_null().primary_key())
                        .col(ColumnDef::new(Groups::SocialMediaId).big_integer().not_null())
                        .col(ColumnDef::new(Groups::Name).string().not_null())
                        .col(ColumnDef::new(Groups::Joined).timestamp_with_time_zone().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_groups_social_media_id")
                                .from(Groups::Table, Groups::SocialMediaId)
                                .to(super::m20231121_000012_create_social_media_table::SocialMedia::Table, super::m20231121_000012_create_social_media_table::SocialMedia::Id)
                                .on_delete(ForeignKeyAction::Cascade)
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Groups::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum Groups {
        Table,
        Id,
        SocialMediaId,
        Name,
        Joined,
    }
}

pub mod m20231121_000015_create_large_data_table {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(LargeData::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(LargeData::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(LargeData::UserId).big_integer().not_null())
                        .col(ColumnDef::new(LargeData::Blob1).string().not_null())
                        .col(ColumnDef::new(LargeData::Blob2).string().not_null())
                        .col(ColumnDef::new(LargeData::Blob3).string().not_null())
                        .col(ColumnDef::new(LargeData::Blob4).string().not_null())
                        .col(ColumnDef::new(LargeData::Blob5).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_large_data_user_id")
                                .from(LargeData::Table, LargeData::UserId)
                                .to(
                                    super::m20231121_000001_create_users_table::Users::Table,
                                    super::m20231121_000001_create_users_table::Users::Id,
                                )
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(LargeData::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    pub enum LargeData {
        Table,
        Id,
        UserId,
        Blob1,
        Blob2,
        Blob3,
        Blob4,
        Blob5,
    }
}

pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    Migrator::up(db, None).await
}
