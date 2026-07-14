//! 会话状态与收束原因（DB `conversations.status` / `winding_reason`）。

use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::mysql::{MySqlTypeInfo, MySqlValueRef};
use sqlx::{Decode, Encode, MySql, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationStatus {
    Active,
    WindingDown,
    /// 话题正常结束，不必为同一话题重启。
    Paused,
    /// 角色去忙导致异常中断。
    PausedCharBusy,
    /// 用户去忙导致异常中断。
    PausedUserBusy,
}

impl ConversationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::WindingDown => "winding_down",
            Self::Paused => "paused",
            Self::PausedCharBusy => "paused_char_busy",
            Self::PausedUserBusy => "paused_user_busy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "winding_down" => Some(Self::WindingDown),
            "paused" => Some(Self::Paused),
            "paused_char_busy" => Some(Self::PausedCharBusy),
            "paused_user_busy" => Some(Self::PausedUserBusy),
            _ => None,
        }
    }

    pub fn is_terminal_pause(self) -> bool {
        matches!(
            self,
            Self::Paused | Self::PausedCharBusy | Self::PausedUserBusy
        )
    }
}

impl std::fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Type<MySql> for ConversationStatus {
    fn type_info() -> MySqlTypeInfo {
        <str as Type<MySql>>::type_info()
    }

    fn compatible(ty: &MySqlTypeInfo) -> bool {
        <str as Type<MySql>>::compatible(ty)
    }
}

impl<'r> Decode<'r, MySql> for ConversationStatus {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as Decode<'r, MySql>>::decode(value)?;
        Self::parse(s).ok_or_else(|| format!("invalid conversation status: {s}").into())
    }
}

impl Encode<'_, MySql> for ConversationStatus {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        <&str as Encode<MySql>>::encode_by_ref(&self.as_str(), buf)
    }
}

/// `winding_down` 期间记录的收束原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingReason {
    Normal,
    CharBusy,
    UserBusy,
}

impl WindingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::CharBusy => "char_busy",
            Self::UserBusy => "user_busy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "char_busy" => Some(Self::CharBusy),
            "user_busy" => Some(Self::UserBusy),
            _ => None,
        }
    }

    /// 告别 / 超时后的终态。
    pub fn terminal_status(self) -> ConversationStatus {
        match self {
            Self::Normal => ConversationStatus::Paused,
            Self::CharBusy => ConversationStatus::PausedCharBusy,
            Self::UserBusy => ConversationStatus::PausedUserBusy,
        }
    }
}

impl std::fmt::Display for WindingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Type<MySql> for WindingReason {
    fn type_info() -> MySqlTypeInfo {
        <str as Type<MySql>>::type_info()
    }

    fn compatible(ty: &MySqlTypeInfo) -> bool {
        <str as Type<MySql>>::compatible(ty)
    }
}

impl<'r> Decode<'r, MySql> for WindingReason {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as Decode<'r, MySql>>::decode(value)?;
        Self::parse(s).ok_or_else(|| format!("invalid winding_reason: {s}").into())
    }
}

impl Encode<'_, MySql> for WindingReason {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        <&str as Encode<MySql>>::encode_by_ref(&self.as_str(), buf)
    }
}
