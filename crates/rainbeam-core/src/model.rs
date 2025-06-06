use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use hcaptcha_no_wasm::Hcaptcha;
use serde::{Deserialize, Serialize};

use authbeam::model::{IpBlock, Profile, UserFollow};
use databeam::prelude::*;
pub use authbeam::model::RelationshipStatus;
use carp::CarpGraph;

/// Trait for simple asset contexts
pub trait Context {}

/// Trait for generic structs which contain a "content" and "context"
pub trait CtxAsset {
    fn ref_context(&self) -> &impl Context;
    fn ref_content(&self) -> &String;
    fn ref_asset(&self) -> (AssetType, &String);
}

/// A question structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    /// The author of the question; "anonymous" marks the question as an anonymous question
    pub author: Box<Profile>,
    /// The recipient of the question; cannot be anonymous
    pub recipient: Box<Profile>,
    /// The content of the question
    pub content: String,
    /// The ID of the question
    pub id: String,
    /// The IP address of the user asking the question
    #[serde(default)]
    pub ip: String,
    /// The time this question was asked
    pub timestamp: u128,
    /// Additional information about the question
    #[serde(default)]
    pub context: QuestionContext,
}

impl Question {
    pub fn get_real_id(&self) -> String {
        if !self.context.source_id.is_empty() {
            return self.context.source_id.clone();
        }

        return self.id.clone();
    }
}

impl CtxAsset for Question {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Question, &self.content)
    }
}

impl Question {
    pub fn lost(author: String, recipient: String, content: String, timestamp: u128) -> Self {
        Self {
            author: anonymous_profile(author),
            recipient: anonymous_profile(recipient),
            content,
            id: "76f75e6129fe30135bd44d80ab7cc46fdba81907758dc808f3e2517beef2b1e9".to_string(), // lost
            ip: String::new(),
            timestamp,
            context: QuestionContext::default(),
        }
    }

    pub fn post() -> Self {
        Self {
            author: anonymous_profile("anonymous".to_string()),
            recipient: anonymous_profile("anonymous".to_string()),
            content: "<post>".to_string(),
            id: "0".to_string(),
            ip: String::new(),
            timestamp: 0,
            context: QuestionContext::default(),
        }
    }

    pub fn unknown() -> Self {
        Self::lost(
            "anonymous".to_string(),
            String::new(),
            "<lost question>".to_string(),
            0,
        )
    }

    pub fn render_media(&self) -> String {
        if self.context.media.is_empty() | (self.context.media == "0") {
            return String::new();
        }

        if self.context.media.starts_with("--CARP") {
            if let Ok(g) =
                carp::carp1::Graph::from_str(self.context.media.replace("--CARP", "").as_str())
            {
                return g.to_svg();
            }
        } else {
            return format!(
                "<img src=\"/api/v1/questions/{}/media.svg\" loading=\"lazy\" class=\"carpgraph\" style=\"width: 300px; height: 200px\" />",
                self.id
            );
        }

        String::new()
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionContext {
    /// The media property of the question
    ///
    /// Media is prefixed to decide what its type is:
    /// * `(no prefix)`: carp2 drawing
    /// * `--CARP`: carp canvas drawing
    // #[deprecated = "use carp2"]
    #[serde(default)]
    pub media: String,
    /// The real ID of this question (for aliasing a global question).
    #[serde(default)]
    pub ref_id: String,
    /// The source ID of this question (if pulled from a ref_id).
    #[serde(default)]
    pub source_id: String,
}

impl Context for QuestionContext {}

impl Default for QuestionContext {
    fn default() -> Self {
        Self {
            media: String::new(),
            ref_id: String::new(),
            source_id: String::new(),
        }
    }
}

/// A question structure with ID references to profiles instead of the profiles
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RefQuestion {
    /// The author of the question; "anonymous" marks the question as an anonymous question
    pub author: String,
    /// The recipient of the question; cannot be anonymous
    pub recipient: String,
    /// The content of the question
    pub content: String,
    /// The ID of the question
    pub id: String,
    /// The IP address of the user asking the questionn
    pub ip: String,
    /// The time this question was asked
    pub timestamp: u128,
    /// Additional information about the question
    #[serde(default)]
    pub context: QuestionContext,
}

impl From<Question> for RefQuestion {
    fn from(value: Question) -> Self {
        Self {
            author: value.author.id,
            recipient: value.recipient.id,
            content: value.content,
            id: value.id,
            ip: value.ip,
            timestamp: value.timestamp,
            context: value.context,
        }
    }
}

/// A response structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionResponse {
    /// The author of the response; cannot be anonymous
    pub author: Box<Profile>,
    /// The ID question this response is replying to
    pub question: String,
    /// The content of the response
    pub content: String,
    /// The ID of the response
    pub id: String,
    /// The time this response was created
    pub timestamp: u128,
    /// The response tags
    pub tags: Vec<String>,
    /// Response context
    pub context: ResponseContext,
    /// The ID of the response this response is replying to
    pub reply: String,
    /// The time this response was last edited
    pub edited: u128,
}

impl QuestionResponse {
    pub fn empty() -> Self {
        Self {
            author: Box::new(authbeam::model::Profile::global()),
            question: String::new(),
            content: String::new(),
            id: String::new(),
            timestamp: 0,
            tags: Vec::new(),
            context: ResponseContext::default(),
            reply: String::new(),
            edited: 0,
        }
    }
}

impl CtxAsset for QuestionResponse {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Response, &self.content)
    }
}

pub type FullResponse = (Question, QuestionResponse, usize, usize);

impl CtxAsset for FullResponse {
    fn ref_context(&self) -> &impl Context {
        &self.1.context
    }

    fn ref_content(&self) -> &String {
        &self.1.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Response, &self.1.content)
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseContext {
    /// If the response is unlisted (not shown on PUBLIC timelines/searches)
    #[serde(default)]
    pub unlisted: bool,
    /// The warning shown on the response. Users must accept this warning to view the response
    ///
    /// Empty means no warning.
    #[serde(default)]
    pub warning: String,
}

impl Context for ResponseContext {}

impl Default for ResponseContext {
    fn default() -> Self {
        Self {
            unlisted: false,
            warning: String::new(),
        }
    }
}

/// A comment structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseComment {
    /// The author of the comment; cannot be anonymous
    pub author: Box<Profile>,
    /// ID of the response this comment is replying to
    pub response: String,
    /// The content of the comment
    pub content: String,
    /// The ID of the comment
    pub id: String,
    /// The time this comment was created
    pub timestamp: u128,
    /// The ID of the comment this comment is replying to
    pub reply: Option<Box<ResponseComment>>,
    /// The time this comment was last edited
    pub edited: u128,
    /// The IP address of the user creating the comment
    #[serde(default)]
    pub ip: String,
    /// Extra information about the comment
    #[serde(default)]
    pub context: CommentContext,
}

impl CtxAsset for ResponseComment {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Comment, &self.content)
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommentContext {}

impl Context for CommentContext {}
impl Default for CommentContext {
    fn default() -> Self {
        Self {}
    }
}

/// A reaction structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reaction {
    /// The reactor of the reaction; cannot be anonymous
    pub user: Box<Profile>,
    /// ID of the asset this reaction is on (response, comment, etc.)
    pub asset: String,
    /// The time this reaction was created
    pub timestamp: u128,
}

/// The type of any asset (anything created by a user)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssetType {
    /// A [`Question`]
    Question,
    /// A [`QuestionResponse`]
    Response,
    /// A [`ResponseComment`]
    Comment,
    /// A market item
    Item,
}

/// The status of a user's membership in a [`Circle`]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MembershipStatus {
    /// A user who has received an invite to a circle, but has not yet accepted
    Pending,
    /// An active member of a circle
    Active,
    /// An active moderator of a circle
    Moderator,
    /// Not pending or an active member
    Inactive,
}

/// The stored version of a user's membership in a [`Circle`]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CircleMembership {
    /// The ID of the user
    pub user: String,
    /// The ID of the circle
    pub circle: String,
    /// The status of the user's membership in the circle
    pub membership: MembershipStatus,
    /// The time the membership was last updated
    pub timestamp: u128,
}

/// A circle structure
///
/// Circles allow you to post global questions to them (recipient `@circle`),
/// as well as define a custom avatar URL, banner URL, and define a custom theme
///
/// Users can also ask a question and send it to the circle's inbox.
/// This question can then be replied to by anybody in the circle.
///
/// Users can be invited to a circle by the circle's owner. Users are added to the `xcircle_memberships`
/// table with a [`MembershipStatus`] of `Pending`. Users can accept through a notification that is sent
/// to their account, which will then change their [`MembershipStatus`] to `Active`.
///
/// Active members can post to the circle through the compose form. Memberships can always be managed
/// by the owner of the circle, who can remove anybody they want from the circle.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Circle {
    /// The name of the circle
    pub name: String,
    /// The ID of the circle
    pub id: String,
    /// The owner of the circle
    pub owner: Box<Profile>,
    /// The metadata of the circle
    pub metadata: CircleMetadata,
    /// The time the circle was created
    pub timestamp: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CircleMetadata {
    pub kv: HashMap<String, String>,
}

impl CircleMetadata {
    /// Check `kv` lengths
    ///
    /// # Returns
    /// * `true`: ok
    /// * `false`: invalid
    pub fn check(&self) -> bool {
        for field in &self.kv {
            if field.0 == "sparkler:custom_css" {
                // custom_css gets an extra long value
                if field.1.len() > 64 * 128 {
                    return false;
                }

                continue;
            }

            if field.1.len() > 64 * 64 {
                return false;
            }
        }

        true
    }
}

/// An export of a user's entire history
#[derive(Serialize, Deserialize)]
pub struct DataExport {
    /// The user's profile
    #[serde(default)]
    pub profile: Box<Profile>,
    /// All of the user's [`Question`]s
    #[serde(default)]
    pub questions: Option<Vec<(Question, usize, usize)>>,
    /// All of the user's [`QuestionResponse`]s
    #[serde(default)]
    pub responses: Option<Vec<FullResponse>>,
    /// All of the user's [`ResponseComment`]s
    #[serde(default)]
    pub comments: Option<Vec<(ResponseComment, usize, usize)>>,
    /// Get all of the user's ipblocks
    #[serde(default)]
    pub ipblocks: Option<Vec<IpBlock>>,
    /// Get all of the user's relationships
    #[serde(default)]
    pub relationships: Option<Vec<(Box<Profile>, RelationshipStatus)>>,
    /// Get all of the user's following
    #[serde(default)]
    pub following: Option<Vec<(UserFollow, Box<Profile>, Box<Profile>)>>,
    /// Get all of the user's followers
    #[serde(default)]
    pub followers: Option<Vec<(UserFollow, Box<Profile>, Box<Profile>)>>,
}

#[derive(Serialize, Deserialize)]
pub struct DataExportOptions {
    /// Include all
    #[serde(default)]
    pub all: bool,
    /// Include `questions`
    #[serde(default)]
    pub questions: bool,
    /// Include `responses`
    #[serde(default)]
    pub responses: bool,
    /// Include `comments`
    #[serde(default)]
    pub comments: bool,
    /// Include `ipblocks`
    #[serde(default)]
    pub ipblocks: bool,
    /// Include `relationships`
    #[serde(default)]
    pub relationships: bool,
    /// Include `followers`
    #[serde(default)]
    pub followers: bool,
    /// Include `following`
    #[serde(default)]
    pub following: bool,
}

// ...

/// Anonymous user profile
pub fn anonymous_profile(tag: String) -> Box<Profile> {
    Box::new(Profile::anonymous(tag))
}

// props

#[derive(Serialize, Deserialize, Debug)]
pub struct QuestionCreate {
    pub recipient: String,
    pub content: String,
    pub anonymous: bool,
    #[serde(default)]
    pub media: Vec<u8>,
    #[serde(default)]
    pub ref_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseCreate {
    pub question: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub warning: String,
    #[serde(default)]
    pub reply: String,
    #[serde(default)]
    pub unlisted: bool,
    #[serde(default)]
    pub circle: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEdit {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEditTags {
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEditTagsMultiple {
    pub ids: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseDeleteMultiple {
    pub ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEditContext {
    pub context: ResponseContext,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEditWarning {
    pub warning: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommentCreate {
    pub response: String,
    pub content: String,
    #[serde(default)]
    pub reply: String,
    #[serde(default)]
    pub anonymous: bool,
}

#[derive(Serialize, Deserialize, Debug, Hcaptcha)]
pub struct CircleCreate {
    pub name: String,
    #[captcha]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EditCircleMetadata {
    pub metadata: CircleMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReactionCreate {
    pub r#type: AssetType,
}

/// General API errors
#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseError {
    AnonymousNotAllowed,
    InvalidNameUnique,
    ContentTooShort,
    ContentTooLong,
    ProfileLocked,
    InvalidName,
    NotAllowed,
    ValueError,
    NotFound,
    Blocked,
    Banned,
    Other,
}

impl DatabaseError {
    pub fn to_string(&self) -> String {
        use DatabaseError::*;
        match self {
            AnonymousNotAllowed => {
                String::from("This profile is not currently accepting anonymous questions.")
            }
            InvalidNameUnique => String::from("This name cannot be used as it is already in use."),
            ContentTooShort => String::from("Content too short!"),
            ContentTooLong => String::from("Content too long!"),
            ProfileLocked => String::from("This profile is not currently accepting questions."),
            InvalidName => String::from("This name cannot be used!"),
            NotAllowed => String::from("You are not allowed to do this!"),
            ValueError => String::from("One of the field values given is invalid!"),
            NotFound => {
                String::from("Nothing with this path exists or you do not have access to it!")
            }
            Blocked => String::from("You're blocked."),
            Banned => String::from("You're banned for suspected systems abuse or violating TOS."),
            _ => String::from("An unspecified error has occured"),
        }
    }

    pub fn to_json<T: Default>(&self) -> DefaultReturn<T> {
        DefaultReturn {
            success: false,
            message: self.to_string(),
            payload: T::default(),
        }
    }
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        use DatabaseError::*;
        match self {
            NotAllowed => (
                StatusCode::UNAUTHORIZED,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 401,
                }),
            )
                .into_response(),
            NotFound => (
                StatusCode::NOT_FOUND,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 404,
                }),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 500,
                }),
            )
                .into_response(),
        }
    }
}

impl<T: Default> Into<DefaultReturn<T>> for DatabaseError {
    fn into(self) -> DefaultReturn<T> {
        DefaultReturn {
            success: false,
            message: self.to_string(),
            payload: T::default(),
        }
    }
}

impl From<authbeam::model::DatabaseError> for DatabaseError {
    fn from(_: authbeam::model::DatabaseError) -> Self {
        Self::Other
    }
}
