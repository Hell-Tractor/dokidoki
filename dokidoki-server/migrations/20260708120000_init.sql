-- Dokidoki MVP schema
-- MySQL 8.0+

CREATE TABLE users (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    username        VARCHAR(64)  NOT NULL,
    password_hash   VARCHAR(255) NOT NULL,
    display_name    VARCHAR(64)  NOT NULL,
    birthday        DATE         NULL,
    max_proactive_per_day INT    NOT NULL DEFAULT 20,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE KEY uk_users_username (username)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_sessions (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    user_id         CHAR(36)     NOT NULL,
    token_hash      CHAR(64)     NOT NULL COMMENT 'SHA-256 hex of Bearer token',
    expires_at      DATETIME(6)  NULL COMMENT 'NULL = no expiry (MVP)',
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE KEY uk_sessions_token_hash (token_hash),
    KEY idx_sessions_user_id (user_id),
    CONSTRAINT fk_sessions_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE characters (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    name            VARCHAR(64)  NOT NULL,
    avatar_path     VARCHAR(255) NULL,
    persona_json    JSON         NOT NULL,
    schedule_json   JSON         NOT NULL,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_character_settings (
    user_id         CHAR(36)     NOT NULL,
    character_id    CHAR(36)     NOT NULL,
    dnd_start       TIME         NULL,
    dnd_end         TIME         NULL,
    push_muted      TINYINT(1)   NOT NULL DEFAULT 0,
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id, character_id),
    CONSTRAINT fk_ucs_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_ucs_character FOREIGN KEY (character_id) REFERENCES characters (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE conversations (
    id                  CHAR(36)     NOT NULL PRIMARY KEY,
    user_id             CHAR(36)     NOT NULL,
    character_id        CHAR(36)     NOT NULL,
    status              ENUM('active', 'winding_down', 'paused') NOT NULL DEFAULT 'active',
    paused_at           DATETIME(6)  NULL,
    summary             TEXT         NULL,
    first_contact_done  TINYINT(1)   NOT NULL DEFAULT 0,
    created_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE KEY uk_conversations_user_character (user_id, character_id),
    KEY idx_conversations_user_id (user_id),
    CONSTRAINT fk_conversations_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_conversations_character FOREIGN KEY (character_id) REFERENCES characters (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE messages (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    conversation_id CHAR(36)     NOT NULL,
    role            ENUM('user', 'character') NOT NULL,
    content         TEXT         NULL,
    content_type    ENUM('text', 'image') NOT NULL DEFAULT 'text',
    turn_id         CHAR(36)     NULL,
    seq_in_turn     INT          NOT NULL DEFAULT 0,
    is_burst_part   TINYINT(1)   NOT NULL DEFAULT 0,
    image_path      VARCHAR(255) NULL,
    reply_to_id     CHAR(36)     NULL,
    read_at         DATETIME(6)  NULL,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    KEY idx_messages_conversation_created (conversation_id, created_at),
    KEY idx_messages_turn (turn_id, seq_in_turn),
    CONSTRAINT fk_messages_conversation FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE,
    CONSTRAINT fk_messages_reply_to FOREIGN KEY (reply_to_id) REFERENCES messages (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 按 character_id 全局唯一：同一角色的日程状态所有用户共享
CREATE TABLE character_states (
    character_id        CHAR(36)     NOT NULL PRIMARY KEY,
    current_activity    VARCHAR(128) NOT NULL DEFAULT '',
    current_mood        VARCHAR(64)  NOT NULL DEFAULT '',
    availability        ENUM('low', 'medium', 'high') NOT NULL DEFAULT 'medium',
    activity_ends_at    DATETIME(6)  NULL,
    random_event        VARCHAR(255) NULL,
    random_event_date   DATE         NULL,
    last_proactive_at   DATETIME(6)  NULL,
    updated_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_character_states_character FOREIGN KEY (character_id) REFERENCES characters (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_memories (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    user_id         CHAR(36)     NOT NULL,
    character_id    CHAR(36)     NOT NULL,
    content         TEXT         NOT NULL,
    memory_type     ENUM('trivial', 'normal', 'important', 'permanent') NOT NULL DEFAULT 'normal',
    memory_key      VARCHAR(64)  NULL,
    expires_at      DATETIME(6)  NULL,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    KEY idx_memories_user_character (user_id, character_id),
    KEY idx_memories_expires (user_id, character_id, expires_at),
    UNIQUE KEY uk_memories_key (user_id, character_id, memory_key),
    CONSTRAINT fk_memories_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_memories_character FOREIGN KEY (character_id) REFERENCES characters (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE proactive_logs (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    user_id         CHAR(36)     NOT NULL,
    character_id    CHAR(36)     NOT NULL,
    conversation_id CHAR(36)     NULL,
    trigger_type    VARCHAR(32)  NOT NULL,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    KEY idx_proactive_logs_user_day (user_id, created_at),
    CONSTRAINT fk_proactive_logs_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_proactive_logs_character FOREIGN KEY (character_id) REFERENCES characters (id) ON DELETE CASCADE,
    CONSTRAINT fk_proactive_logs_conversation FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE devices (
    id              CHAR(36)     NOT NULL PRIMARY KEY,
    user_id         CHAR(36)     NOT NULL,
    fcm_token       VARCHAR(512) NOT NULL,
    platform        VARCHAR(16)  NOT NULL DEFAULT 'android',
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE KEY uk_devices_user_token (user_id, fcm_token),
    CONSTRAINT fk_devices_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
