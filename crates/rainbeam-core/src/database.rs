use async_recursion::async_recursion;
use authbeam::ignore;
use pathbufd::pathd;
use rainbeam_shared::snow::AlmostSnowflake;
use std::collections::{BTreeMap, HashMap};

use crate::config::Config;
use crate::model::*;
use crate::model::{DatabaseError, Question};

use authbeam::{
    simplify, from_row,
    model::{FinePermission, NotificationCreate, Profile, RelationshipStatus},
};
use databeam::{utility, query as sqlquery, prelude::*};
use langbeam::LangFile;

pub type Result<T> = std::result::Result<T, DatabaseError>;

/// Database connector
#[derive(Clone)]
pub struct Database {
    pub base: StarterDatabase,
    pub auth: authbeam::Database,
    pub config: Config,
    langs: HashMap<String, LangFile>,
}

impl Database {
    pub async fn new(
        opts: databeam::DatabaseOpts,
        auth: authbeam::Database,
        config: Config,
    ) -> Self {
        Self {
            base: StarterDatabase::new(opts).await,
            auth,
            config,
            langs: langbeam::read_langs(),
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        // create questions table
        // we're only going to store unanswered questions here
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xquestions\" (
                author    TEXT,
                recipient TEXT,
                content   TEXT,
                id        TEXT,
                timestamp TEXT,
                ip        TEXT,
                context   TEXT
            )",
        )
        .execute(c)
        .await;

        // create responses table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xresponses\" (
                author         TEXT,
                question       TEXT,
                content        TEXT,
                id             TEXT,
                timestamp      TEXT,
                tags           TEXT,
                context        TEXT,
                reply          TEXT,
                edited         TEXT,
                reaction_count TEXT DEFAULT \"0\"
            )",
        )
        .execute(c)
        .await;

        // create comments table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xcomments\" (
                author    TEXT,
                response  TEXT,
                content   TEXT,
                id        TEXT,
                timestamp TEXT,
                reply     TEXT,
                edited    TEXT,
                ip        TEXT,
                context   TEXT
            )",
        )
        .execute(c)
        .await;

        // create reactions table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xreactions\" (
                user      TEXT,
                asset     TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;
    }

    // language

    /// Get a [`LangFile`] given its ID
    ///
    /// Returns `net.rainbeam.langs:en-US` if the given file cannot be found.
    pub fn lang(&self, id: &str) -> LangFile {
        if id.is_empty() {
            // don't even try to fetch an empty id
            return self
                .langs
                .get("net.rainbeam.langs:en-US")
                .unwrap()
                .to_owned();
        } else if (id == "aa-BB") | (id == "net.rainbeam.langs.testing:aa-BB") {
            // debug
            return LangFile::default();
        }

        self.langs
            .get(id)
            .unwrap_or(self.langs.get("net.rainbeam.langs:en-US").unwrap())
            .to_owned()
    }

    // anonymous tag

    /// Get the tag of an anonymous username
    ///
    /// # Returns
    /// `(is anonymous, tag, username, input)`
    pub fn anonymous_tag(input: &str) -> (bool, String, String, String) {
        Profile::anonymous_tag(input)
    }

    /// Create an anonymous username
    ///
    /// # Returns
    /// `("anonymous#" + tag, tag)`
    pub fn create_anonymous(&self) -> (String, String) {
        let tag = rainbeam_shared::hash::random_id();
        (format!("anonymous#{tag}"), tag)
    }

    // profiles

    /// Fetch a profile correctly
    pub async fn get_profile<T: AsRef<str>>(&self, id: T) -> Result<Box<Profile>> {
        match self.auth.get_profile(id.as_ref()).await {
            Ok(p) => Ok(p),
            Err(e) => Err(e.into()),
        }
    }

    /// Parse user mentions in a given `input` String
    pub fn parse_mentions(input: String) -> Vec<String> {
        // state
        let mut escape: bool = false;
        let mut at: bool = false;
        let mut buffer: String = String::new();
        let mut out = Vec::new();

        // parse
        for char in input.chars() {
            if (char == '\\') && !escape {
                escape = true;
            }

            if (char == '@') && !escape {
                at = true;
                continue; // don't push @
            }

            if at {
                if (char == ' ') && !escape {
                    // reached space, end @
                    at = false;

                    if !out.contains(&buffer) {
                        out.push(buffer);
                    }

                    buffer = String::new();
                    continue;
                }

                // push mention text
                buffer.push(char);
            }

            escape = false;
        }

        // return
        out
    }

    /// Get all profiles by a search query, 12 at a time
    ///
    /// # Arguments
    /// * `page`
    /// * `search`
    pub async fn get_profiles_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<Box<Profile>>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT \"id\" FROM \"xprofiles\" WHERE \"username\" LIKE ? ORDER BY \"joined\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT \"id\" FROM \"xprofiles\" WHERE \"username\" LIKE $1 ORDER BY \"joined\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Box<Profile>> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push(match self.get_profile(id).await {
                        Ok(p) => p,
                        Err(_) => continue,
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Export all data of the given `user`
    pub async fn create_data_export(
        &self,
        user: String,
        options: DataExportOptions,
    ) -> Result<DataExport> {
        Ok(DataExport {
            profile: match self.get_profile(user.clone()).await {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
            questions: if options.questions | options.all {
                match self.get_questions_by_author(user.clone()).await {
                    Ok(r) => Some(r),
                    Err(e) => return Err(e),
                }
            } else {
                None
            },
            responses: if options.responses | options.all {
                match self.get_responses_by_author(&user).await {
                    Ok(r) => Some(r),
                    Err(e) => return Err(e),
                }
            } else {
                None
            },
            comments: if options.comments | options.all {
                match self.get_comments_by_author(user.clone()).await {
                    Ok(r) => Some(r),
                    Err(e) => return Err(e),
                }
            } else {
                None
            },
            ipblocks: if options.ipblocks | options.all {
                match self.auth.get_ipblocks(&user).await {
                    Ok(r) => Some(r),
                    Err(_) => return Err(DatabaseError::Other),
                }
            } else {
                None
            },
            relationships: if options.relationships | options.all {
                match self.auth.get_user_relationships(&user).await {
                    Ok(r) => Some(r),
                    Err(_) => return Err(DatabaseError::Other),
                }
            } else {
                None
            },
            following: if options.following | options.all {
                match self.auth.get_following(&user).await {
                    Ok(r) => Some(r),
                    Err(_) => return Err(DatabaseError::Other),
                }
            } else {
                None
            },
            followers: if options.followers | options.all {
                match self.auth.get_followers(&user).await {
                    Ok(r) => Some(r),
                    Err(_) => return Err(DatabaseError::Other),
                }
            } else {
                None
            },
        })
    }

    // extra util

    /// Create a moderator audit log entry
    pub async fn audit(&self, actor_id: String, content: String) -> Result<()> {
        match self.auth.audit(&actor_id, &content).await {
            Ok(r) => Ok(r),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // questions

    /// Get an existing question
    ///
    /// # Arguments
    /// * `id`
    #[async_recursion]
    pub async fn get_question(&self, id: String) -> Result<Question> {
        if id == "0" {
            return Ok(Question::post());
        }

        // legacy migration
        if id.starts_with("{") {
            let question = serde_json::from_str::<serde_json::Value>(&id).unwrap();

            return Ok(Question {
                author: match self
                    .get_profile(
                        question
                            .get("author")
                            .unwrap()
                            .to_string()
                            .trim_matches(|c| c == '"'),
                    )
                    .await
                {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                },
                recipient: match self
                    .get_profile(
                        question
                            .get("recipient")
                            .unwrap()
                            .to_string()
                            .trim_matches(|c| c == '"'),
                    )
                    .await
                {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                },
                content: question
                    .get("content")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .to_string(),
                id: question
                    .get("id")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .to_string(),
                ip: question
                    .get("id")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .to_string(),
                timestamp: question
                    .get("timestamp")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .parse::<u128>()
                    .unwrap(),
                context: QuestionContext::default(),
            });
        }

        // check in cache
        // we still prefix rainbeam under the "sparkler" name for compatibility with the first 6 development versions
        match self
            .base
            .cache
            .get(format!("rbeam.app.question:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<RefQuestion>(c.as_str()) {
                Ok(q) => {
                    return Ok(Question {
                        author: match self.get_profile(&q.author).await {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile(q.author),
                        },
                        recipient: match self
                            .get_profile(if q.recipient.starts_with("ANSWERED:") {
                                q.recipient.replace("ANSWERED:", "")
                            } else {
                                q.recipient.clone()
                            })
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile(q.recipient),
                        },
                        content: q.content,
                        id: q.id,
                        ip: q.ip,
                        timestamp: q.timestamp,
                        context: q.context,
                    })
                }
                Err(_) => {
                    // remove bad entry and continue to fetch from database
                    self.base
                        .cache
                        .remove(format!("rbeam.app.question:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Ok(Question::unknown()),
        };

        // return
        let mut question = Question {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile(res.get("author").unwrap().to_string()),
            },
            recipient: match self
                .get_profile(res.get("recipient").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile(res.get("recipient").unwrap().to_string()),
            },
            content: from_row!(res->content()),
            id: from_row!(res->id()),
            ip: from_row!(res->ip()),
            timestamp: from_row!(res->timestamp(u128); 0),
            context: from_row!(res->context(json); DatabaseError::ValueError),
        };

        // check ref question
        if !question.context.ref_id.is_empty() {
            let q = simplify!(self.get_question(question.context.ref_id.clone()).await; Result);
            let original_id = question.id.clone();
            question = q;
            question.context.source_id = original_id;
        }

        // store in cache
        if id.len() == 64 {
            self.base
                .cache
                .set(
                    format!("rbeam.app.question:{}", id),
                    serde_json::to_string::<RefQuestion>(&RefQuestion::from(question.clone()))
                        .unwrap(),
                )
                .await;
        }

        // return
        Ok(question)
    }

    /// Get all questions by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    pub async fn get_questions_by_recipient(&self, recipient: &str) -> Result<Vec<Question>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"recipient\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"recipient\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Question> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let mut question = Question {
                        author: match self
                            .get_profile(res.get("author").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile("anonymous".to_string()),
                        },
                        recipient: match self
                            .get_profile(res.get("recipient").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile("anonymous".to_string()),
                        },
                        content: res.get("content").unwrap().to_string(),
                        id: from_row!(res->id()),
                        ip: from_row!(res->ip()),
                        timestamp: from_row!(res->timestamp(u128); 0),
                        context: from_row!(res->context(json); DatabaseError::ValueError),
                    };

                    // check ref question
                    if !question.context.ref_id.is_empty() {
                        let q = simplify!(self.get_question(question.context.ref_id.clone()).await; Result);
                        let original_id = question.id.clone();
                        question = q;
                        question.context.source_id = original_id;
                    }

                    // ...
                    out.push(question);
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of notifications by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    pub async fn get_inbox_count_by_recipient(&self, recipient: &str) -> usize {
        match self.get_profile(recipient).await {
            Ok(x) => x.inbox_count,
            Err(_) => 0,
        }
    }

    /// Get all questions by their author, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `page`
    pub async fn get_questions_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<Question>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? OR \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 OR \"author\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("anonymous#{}", author.to_lowercase()))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Question> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(Question {
                        author: match self
                            .get_profile(res.get("author").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        recipient: match self
                            .get_profile(res.get("recipient").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        content: res.get("content").unwrap().to_string(),
                        id: from_row!(res->id()),
                        ip: from_row!(res->ip()),
                        timestamp: from_row!(res->timestamp(u128); 0),
                        context: from_row!(res->context(json); DatabaseError::ValueError),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_global_questions_by_author(
        &self,
        author: &str,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author, 20 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `page`
    pub async fn get_global_questions_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author and a search query, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `search`
    /// * `page`
    pub async fn get_global_questions_by_author_searched_paginated(
        &self,
        author: String,
        search: String,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' AND \"content\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by a search query, 12 at a time
    ///
    /// # Arguments
    /// * `page`
    /// * `search`
    pub async fn get_global_questions_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' AND \"content\" LIKE $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get global questions from people `user` is following, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    pub async fn get_global_questions_by_following_paginated(
        &self,
        user: &str,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // get following
        let following = match self.auth.get_following(&user).await {
            Ok(f) => f,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // check user permissions
        // returning NotAllowed here will block them from viewing their global questions timeline
        // we don't want to waste resources on rule breakers
        let user = match self.auth.get_profile_by_username(&user).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // build string
        let mut query_string = String::new();

        for follow in following {
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.2.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xquestions\" WHERE (\"author\" = ?{query_string}) AND \"recipient\" = '@'  ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!( "SELECT * FROM \"xquestions\" WHERE (\"author\" = $1{query_string}) AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all global questions, 12 at a time
    ///
    /// # Arguments
    /// * `page`
    pub async fn get_global_questions_paginated(
        &self,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!( "SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of global questions by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_global_questions_count_by_author(&self, author: &str) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.global_questions_count:{}", author))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_global_questions_by_author(author)
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(
                format!("rbeam.app.global_question_count:{}", author),
                count.to_string(),
            )
            .await;

        count
    }

    /// Get all questions by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_questions_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = ? OR \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = $1 OR \"author\" = $2 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("anonymous#{}", author.to_lowercase()))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let id = from_row!(res->id());
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => continue,
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: from_row!(res->ip()),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of responses by their question
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_response_count_by_question(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.question_response_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_responses_by_question(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(
                format!("rbeam.app.question_response_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new question
    ///
    /// # Arguments
    /// * `props` - [`QuestionCreate`]
    /// * `author` - the ID of the user creating the question
    /// * `ip` - author IP
    #[async_recursion]
    pub async fn create_question(
        &self,
        mut props: QuestionCreate,
        author: String,
        ip: String,
    ) -> Result<String> {
        // check media
        if props.media.len() > (64 * 512) {
            return Err(DatabaseError::ContentTooLong);
        }

        // check recipient
        // "@" is the recipient we use for global questions (questions anybody can respond to)
        let tag = Database::anonymous_tag(&author);
        let mut use_tier = 0;
        if props.recipient != "@" {
            // profile
            let recipient = match self.get_profile(props.recipient.clone()).await {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            };

            use_tier = recipient.tier;

            let profile_locked = recipient.metadata.is_true("sparkler:lock_profile");
            let block_anonymous = recipient.metadata.is_true("sparkler:disallow_anonymous");

            if profile_locked {
                return Err(DatabaseError::ProfileLocked);
            }

            if (block_anonymous == true) && (tag.0 == true) {
                return Err(DatabaseError::AnonymousNotAllowed);
            }

            if !tag.0 {
                let author = match self.get_profile(author.clone()).await {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                };

                // bump question count
                if let Err(_) = self
                    .auth
                    .update_profile_question_count(&author.id, author.question_count + 1)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // check relationship
                let relationship = self
                    .auth
                    .get_user_relationship(&recipient.id, &author.id)
                    .await
                    .0;

                if relationship == RelationshipStatus::Blocked {
                    return Err(DatabaseError::Blocked);
                }
            }

            // check if we're ip blocked by the recipient
            if let Ok(_) = self.auth.get_ipblock_by_ip(&ip, &recipient.id).await {
                return Err(DatabaseError::Blocked);
            }

            if self
                .auth
                .get_ipblock_by_ip(&ip, &recipient.id)
                .await
                .is_ok()
            {
                return Err(DatabaseError::Blocked);
            }

            // check filter
            for filter_string in recipient
                .metadata
                .kv
                .get("sparkler:filter")
                .unwrap_or(&"".to_string())
                .split("\n")
            {
                if filter_string.is_empty() | filter_string.starts_with("#") {
                    continue;
                }

                if props
                    .content
                    .to_lowercase()
                    .contains(&filter_string.to_lowercase())
                {
                    // return ok so the client thinks it worked, but really we lied
                    return Ok(String::new());
                }
            }

            // incr recipient inbox count
            simplify!(
                self.auth
                    .update_profile_inbox_count(&recipient.id, recipient.inbox_count + 1)
                    .await;
                Err; Err(DatabaseError::Other)
            )
        } else {
            // anonymous users cannot ask global questions
            if tag.0 == true {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check author permissions
        if tag.0 == false {
            let author = match self.get_profile(author.clone()).await {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            };

            if author.group == -1 {
                // group -1 (even if it exists) is for marking users as banned
                return Err(DatabaseError::NotAllowed);
            }

            // check content length
            if (props.content.trim().len() < 2) && props.ref_id.is_empty() {
                return Err(DatabaseError::ContentTooShort);
            }

            // we get upgraded limit if we have the minimum tier OR if the recipient has it
            if (author.tier >= self.config.tiers.double_limits)
                | (use_tier >= self.config.tiers.double_limits)
            {
                if props.content.len() > (64 * 64) {
                    return Err(DatabaseError::ContentTooLong);
                }
            } else {
                if props.content.len() > (64 * 32) {
                    return Err(DatabaseError::ContentTooLong);
                }
            }
        } else {
            // anonymous users cannot post images
            props.content = props.content.replace("![", "[").replace("<img", "<bimg");

            // check content length
            if tag.1.len() == 36 {
                // this is a user id, fetch author and check their limits
                let author = match self.get_profile(tag.1.clone()).await {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                };

                if author.group == -1 {
                    // group -1 (even if it exists) is for marking users as banned
                    return Err(DatabaseError::NotAllowed);
                }

                // check content length
                if (props.content.trim().len() < 2) && props.ref_id.is_empty() {
                    return Err(DatabaseError::ContentTooShort);
                }

                if (author.tier >= self.config.tiers.double_limits)
                    | (use_tier >= self.config.tiers.double_limits)
                {
                    if props.content.len() > (64 * 64) {
                        return Err(DatabaseError::ContentTooLong);
                    }
                } else {
                    if props.content.len() > (64 * 32) {
                        return Err(DatabaseError::ContentTooLong);
                    }
                }
            } else {
                // true anonymous
                if (props.content.trim().len() < 2) && props.ref_id.is_empty() {
                    return Err(DatabaseError::ContentTooShort);
                }

                if use_tier >= self.config.tiers.double_limits {
                    if props.content.len() > (64 * 64) {
                        return Err(DatabaseError::ContentTooLong);
                    }
                } else {
                    if props.content.len() > (64 * 32) {
                        return Err(DatabaseError::ContentTooLong);
                    }
                }
            }
        }

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&props.content);

        if (markdown.trim().len() == 0) && props.ref_id.is_empty() {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let question = Question {
            author: match self.get_profile(author).await {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            recipient: match self.get_profile(props.recipient.clone()).await {
                Ok(ua) => ua,
                Err(_) => anonymous_profile(format!("anonymous.{}", props.recipient)), // future note: this is anonymous because the recipient will be anonymous for cirles
            },
            content: props.content.trim().to_string(),
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            timestamp: utility::unix_epoch_timestamp(),
            ip: ip.clone(),
            context: QuestionContext {
                media: props.media.len().to_string(),
                ref_id: props.ref_id,
                source_id: String::new(),
            },
        };

        // create question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xquestions\" VALUES (?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xquestions\" VALUES ($1, $2, $3, $4, $5, $6, $7)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&question.author.id)
            .bind::<&String>(&props.recipient)
            .bind::<&String>(&question.content)
            .bind::<&String>(&question.id)
            .bind::<&String>(&question.timestamp.to_string())
            .bind::<&String>(&ip)
            .bind::<&String>(&serde_json::to_string(&question.context).unwrap())
            .execute(c)
            .await
        {
            Ok(_) => {
                // incr questions count
                if question.recipient.username == "@" {
                    self.base
                        .cache
                        .incr(format!(
                            "rbeam.app.global_question_count:{}",
                            question.author.username
                        ))
                        .await;

                    // create ref questions for all friends
                    // we're doing friends here instead of followers because are probably
                    // going to have many more followers than friends
                    if !question
                        .author
                        .has_label(authbeam::model::RESERVED_LABEL_QUARANTINE)
                        | !question
                            .author
                            .metadata
                            .is_true("rainbeam:do_not_send_global_questions_to_friends")
                    {
                        let friends = simplify!(
                            self.auth
                                .get_user_participating_relationships_of_status(&question.author.id, RelationshipStatus::Friends)
                                .await;
                            Result; Vec::new()
                        );

                        for friend in friends {
                            let friend = if friend.0.id == question.author.id {
                                friend.1
                            } else {
                                friend.0
                            };

                            if friend
                                .metadata
                                .is_true("rainbeam:do_not_send_global_questions_to_inbox")
                            {
                                continue;
                            }

                            ignore!(
                                self.create_question(
                                    QuestionCreate {
                                        recipient: friend.id,
                                        content: String::new(),
                                        anonymous: false,
                                        media: Vec::new(),
                                        ref_id: question.id.clone(),
                                    },
                                    question.author.id.clone(),
                                    ip.clone(),
                                )
                                .await
                            );
                        }
                    }
                }

                // upload carpgraph
                if !props.media.is_empty() {
                    std::fs::write(
                        pathd!(
                            "{}/carpgraph/{}.carpgraph",
                            self.config.media_dir,
                            question.id
                        ),
                        props.media,
                    )
                    .expect("failed to write carpgraph image");
                }

                // ...
                return Ok(question.id.clone());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing question
    ///
    /// Questions can only be deleted by their recipient or the user that asked them.
    ///
    /// # Arguments
    /// * `id` - the ID of the question
    /// * `user` - the user doing this
    pub async fn delete_question(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure question exists
        let question = match self.get_question(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        let tag = Database::anonymous_tag(&question.author.id);
        if (user.id != question.recipient.id)
            && (user.id != question.author.id)
            && (user.id != tag.1)
        {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_QUESTIONS) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xquestions\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xquestions\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove all responses if this is a global question
                if question.recipient.username == "@" {
                    // delete responses
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "DELETE FROM \"xresponses\" WHERE \"question\" LIKE ?"
                        } else {
                            "DELETE FROM \"xresponses\" WHERE \"question\" LIKE $1"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&format!("%\"id\":\"{}\"%", question.id))
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };

                    // delete response counter
                    self.base
                        .cache
                        .remove(format!("rbeam.app.question_response_count:{}", question.id))
                        .await;

                    // decr questions count
                    self.base
                        .cache
                        .decr(format!(
                            "rbeam.app.global_question_count:{}",
                            question.author.username
                        ))
                        .await;

                    // clear reactions
                    if let Err(e) = self.clear_reactions(question.id.clone()).await {
                        return Err(e);
                    }
                }

                // remove image
                if !question.context.media.is_empty()
                    && !question.context.media.starts_with("--CARP")
                {
                    let _ = std::fs::remove_file(pathd!(
                        "{}/carpgraph/{}.carpgraph",
                        self.config.media_dir,
                        question.id
                    ));
                }

                // remove from cache
                self.base
                    .cache
                    .remove(format!("rbeam.app.question:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete all questions by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    /// * `user`
    pub async fn delete_questions_by_recipient(
        &self,
        recipient: &str,
        user: Box<Profile>,
    ) -> Result<()> {
        // check username
        if user.id != recipient {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_QUESTIONS) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xquestions\" WHERE \"recipient\" = ?"
        } else {
            "DELETE FROM \"xquestions\" WHERE \"recipient\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        if let Err(_) = sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .execute(c)
            .await
        {
            return Err(DatabaseError::NotFound);
        };

        // return
        Ok(())
    }

    // responses

    /// Get a response from a database result
    pub async fn gimme_response(&self, res: BTreeMap<String, String>) -> Result<FullResponse> {
        let question = res.get("question").unwrap().to_string();
        let id = from_row!(res->id());
        let author = res.get("author").unwrap().to_string();
        let ctx: ResponseContext =
            match serde_json::from_str(res.get("context").unwrap_or(&"{}".to_string())) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            };
        let reaction_count = res.get("reaction_count").unwrap().parse::<usize>().unwrap();

        Ok((
            match self.get_question(question.clone()).await {
                Ok(q) => q,
                Err(_) => Question::unknown(),
            },
            QuestionResponse {
                author: if author.starts_with("{") {
                    // likely serialized author struct
                    let de: Profile = serde_json::from_str(&author).unwrap();

                    match self.get_profile(de.id).await {
                        Ok(ua) => ua,
                        Err(_) => anonymous_profile("anonymous".to_string()),
                    }
                } else {
                    // must just be id, fetch normally
                    match self.get_profile(author).await {
                        Ok(ua) => ua,
                        Err(_) => anonymous_profile("anonymous".to_string()),
                    }
                },
                question,
                content: res.get("content").unwrap().to_string(),
                id: id.to_owned(),
                timestamp: from_row!(res->timestamp(u128); 0),
                tags: match serde_json::from_str(res.get("tags").unwrap()) {
                    Ok(t) => t,
                    Err(_) => return Err(DatabaseError::ValueError),
                },
                context: ctx,
                reply: res.get("reply").unwrap_or(&String::new()).to_string(),
                edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
            },
            self.get_comment_count_by_response(id.clone()).await,
            {
                let count = self.get_reaction_count_by_asset(id.clone()).await;

                if reaction_count != count {
                    // ensure values sync (update the lesser value)
                    if reaction_count > count {
                        // count = reaction_count
                        self.update_response_reaction_count(id, count)
                            .await
                            .unwrap();
                    } else {
                        // reaction_count = count
                        self.base
                            .cache
                            .set(
                                format!("rbeam.app.reaction_count:{}", id),
                                count.to_string(),
                            )
                            .await;
                    }
                }

                count
            },
        ))
    }

    /// Get a (short) response from a database result
    pub async fn gimme_short_response(
        &self,
        res: BTreeMap<String, String>,
    ) -> Result<QuestionResponse> {
        let question = res.get("question").unwrap().to_string();
        let id = from_row!(res->id());
        let author = res.get("author").unwrap().to_string();
        let ctx: ResponseContext =
            match serde_json::from_str(res.get("context").unwrap_or(&"{}".to_string())) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            };

        Ok(QuestionResponse {
            author: if author.starts_with("{") {
                // likely serialized author struct
                let de: Profile = serde_json::from_str(&author).unwrap();

                match self.get_profile(de.id).await {
                    Ok(ua) => ua,
                    Err(_) => anonymous_profile("anonymous".to_string()),
                }
            } else {
                // must just be id, fetch normally
                match self.get_profile(author).await {
                    Ok(ua) => ua,
                    Err(_) => anonymous_profile("anonymous".to_string()),
                }
            },
            question,
            content: res.get("content").unwrap().to_string(),
            id: id.to_owned(),
            timestamp: from_row!(res->timestamp(u128); 0),
            tags: match serde_json::from_str(res.get("tags").unwrap()) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            },
            context: ctx,
            reply: res.get("reply").unwrap_or(&String::new()).to_string(),
            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
        })
    }

    /// Get an existing response
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_response(&self, id: String) -> Result<FullResponse> {
        // check in cache
        match self
            .base
            .cache
            .get(format!("rbeam.app.response:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<BTreeMap<String, String>>(c.as_str()) {
                    Ok(res) => {
                        return Ok(match self.gimme_response(res).await {
                            Ok(r) => r,
                            Err(e) => return Err(e),
                        })
                    }
                    Err(_) => {
                        // we're storing a bad version that couldn't deserialize, we don't need that...
                        self.base
                            .cache
                            .remove(format!("rbeam.app.response:{}", id))
                            .await
                    }
                };
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let response = match self.gimme_response(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cache
                .set(
                    format!("rbeam.app.response:{}", id),
                    serde_json::to_string::<QuestionResponse>(&response.1).unwrap(),
                )
                .await;
        }

        // return
        Ok(response)
    }

    /// Get an existing response (short)
    ///
    /// This method is only for when we need a response and not its question and extra information.
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_response_short(&self, id: String) -> Result<QuestionResponse> {
        // check in cache
        match self
            .base
            .cache
            .get(format!("rbeam.app.response:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<BTreeMap<String, String>>(c.as_str()) {
                    Ok(res) => {
                        return Ok(match self.gimme_short_response(res).await {
                            Ok(r) => r,
                            Err(e) => return Err(e),
                        })
                    }
                    Err(_) => {
                        // we're storing a bad version that couldn't deserialize, we don't need that...
                        self.base
                            .cache
                            .remove(format!("rbeam.app.response:{}", id))
                            .await
                    }
                };
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let response = match self.gimme_short_response(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cache
                .set(
                    format!("rbeam.app.response:{}", id),
                    serde_json::to_string::<QuestionResponse>(&response).unwrap(),
                )
                .await;
        }

        // return
        Ok(response)
    }

    /// Get an existing response by the question ID and response author
    ///
    /// # Arguments
    /// * `question`
    /// * `author`
    pub async fn get_response_by_question_and_author(
        &self,
        question: &str,
        author: &str,
    ) -> Result<FullResponse> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = ? AND \"author\" = ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = $1 AND \"author\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&str>(question)
            .bind::<&str>(author)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(match self.gimme_response(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        })
    }

    /// Get all responses, 12 at a time
    ///
    /// # Arguments
    /// * `page`
    pub async fn get_responses_paginated(&self, page: i32) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" NOT LIKE '%\"unlisted\":true%' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" NOT LIKE '%\"unlisted\":true%' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author(&self, author: &str) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author and content search, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author_searched_paginated(
        &self,
        author: String,
        search: String,
        page: i32,
    ) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 AND \"content\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author and tag, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `tag`
    pub async fn get_responses_by_author_tagged_paginated(
        &self,
        author: String,
        tag: String,
        page: i32,
    ) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? AND \"tags\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 AND \"tags\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%\"{}\"%", tag))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their tag, 12 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `tag`
    pub async fn get_responses_tagged_paginated(
        &self,
        tag: String,
        page: i32,
    ) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"tags\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"tags\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%\"{}\"%", tag))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of responses by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_response_count_by_author(&self, author: &str) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.response_count:{}", author))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_responses_by_author(author)
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(
                format!("rbeam.app.response_count:{}", author),
                count.to_string(),
            )
            .await;

        count
    }

    /// Get all responses, 12 at a time, matching search query
    ///
    /// # Arguments
    /// * `page`
    /// * `search`
    pub async fn get_responses_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"content\" LIKE ? AND \"context\" NOT LIKE '%\"unlisted\":true%' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"content\" LIKE $1 AND \"context\" NOT LIKE '%\"unlisted\":true%' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get 50 responses from people `user` is following
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_responses_by_following_paginated(
        &self,
        user: &str,
        page: i32,
    ) -> Result<Vec<FullResponse>> {
        // check in cache
        // match self
        //     .base
        //     .cachedb
        //     .get_timed::<Vec<FullResponse>, String>(format!(
        //         "rbeam.app.timeline_save.get_responses_by_following_paginated:{}:{}",
        //         user, page
        //     ))
        //     .await
        // {
        //     Some(c) => return Ok(c.1),
        //     None => (),
        // };

        // get following
        let following = match self.auth.get_following(&user).await {
            Ok(f) => f,
            Err(_) => Vec::new(),
        };

        // check user permissions
        // returning NotAllowed here will block them from viewing their timeline
        // we don't want to waste resources on rule breakers
        let user = match self.auth.get_profile_by_id(&user).await {
            Ok(ua) => ua,
            Err(_) => anonymous_profile(self.create_anonymous().1),
        };

        // build string
        let mut query_string = String::new();

        for follow in following {
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.2.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ?{query_string} ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!( "SELECT * FROM \"xresponses\" WHERE \"author\" = $1{query_string} ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        self.base
            .cache
            .set_timed(
                format!(
                    "rbeam.app.timeline_save.get_responses_by_following_paginated:{}:{}",
                    user.id, page
                ),
                res.clone(),
            )
            .await;

        Ok(res)
    }

    /// Get all responses by their question ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_responses_by_question(&self, id: String) -> Result<Vec<FullResponse>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Create a new response
    ///
    /// Responses can only be created for questions where `recipient` matches the given `author`
    ///
    /// # Arguments
    /// * `props` - [`ResponseCreate`]
    /// * `author` - the ID of the user creating the response
    pub async fn create_response(
        &self,
        props: ResponseCreate,
        author: String,
    ) -> Result<QuestionResponse> {
        // make sure the question exists
        let mut question = if props.question != "0" {
            // get question from database
            match self.get_question(props.question.clone()).await {
                Ok(q) => q,
                Err(_) => return Err(DatabaseError::NotFound),
            }
        } else {
            // create post question
            Question::post()
        };

        // check author permissions
        let author = match self.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check content length
        if props.content.trim().len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        if author.tier >= self.config.tiers.double_limits {
            if props.content.len() > (64 * 128) {
                return Err(DatabaseError::ContentTooLong);
            }
        } else {
            if props.content.len() > (64 * 64) {
                return Err(DatabaseError::ContentTooLong);
            }
        }

        // check permissions
        if props.question != "0" {
            // normal questions
            if question.recipient.username != "@" {
                if question.recipient.id != author.id {
                    // cannot respond to a question not asked to us
                    return Err(DatabaseError::NotAllowed);
                }

                // check relationship
                // cannot respond to questions from people who blocked us (or we've blocked)
                let relationship = self
                    .auth
                    .get_user_relationship(&question.author.id, &author.id)
                    .await;

                if relationship.0 == RelationshipStatus::Blocked {
                    return Err(DatabaseError::NotAllowed);
                }

                // make sure we haven't already answered this
                if let Ok(_) = self
                    .get_response_by_question_and_author(&question.id, &author.id)
                    .await
                {
                    return Err(DatabaseError::Other);
                }
            }
            // global questions
            else {
                // TODO: check author privacy settings
                let tag = Database::anonymous_tag(&author.id);

                if tag.0 {
                    // anonymous users cannot answer global questions
                    return Err(DatabaseError::NotAllowed);
                }

                // check relationship
                // cannot respond to questions from people who blocked us (or we've blocked)
                let relationship = self
                    .auth
                    .get_user_relationship(&question.author.id, &author.id)
                    .await;

                if relationship.0 == RelationshipStatus::Blocked {
                    return Err(DatabaseError::NotAllowed);
                }

                // make sure we didn't already answer this
                if let Ok(_) = self
                    .get_response_by_question_and_author(&question.id, &author.id)
                    .await
                {
                    // cannot answer the same global question twice
                    return Err(DatabaseError::NotAllowed);
                };
            };
        } else {
            // check tag
            let tag = Database::anonymous_tag(&author.id);

            if tag.0 {
                // anonymous users cannot create posts
                return Err(DatabaseError::NotAllowed);
            }
        };

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check reply
        if !props.reply.is_empty() {
            if let Err(e) = self.get_response(props.reply.trim().to_string()).await {
                return Err(e);
            }
        }

        // ...
        let timestamp = utility::unix_epoch_timestamp();
        let mut response = QuestionResponse {
            author: author.clone(),
            content: props.content.trim().to_string(),
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            timestamp,
            tags: Vec::new(),
            context: ResponseContext {
                unlisted: props.unlisted,
                warning: props.warning,
            },
            question: question.id,
            reply: props.reply.trim().to_string(),
            edited: timestamp,
        };

        // make sure reply exists
        if !response.reply.is_empty() {
            if let Err(e) = self.get_response(response.reply.clone()).await {
                return Err(e);
            }
        }

        // parse mentions
        for mention in Database::parse_mentions(response.content.clone()) {
            let profile = match self.auth.get_profile(&mention).await {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Err(_) = self
                .auth
                .create_notification(
                    NotificationCreate {
                        title: format!(
                            "[@{}]({}) mentioned you in a response!",
                            author.username, author.id
                        ),
                        content: format!("You were mentioned in a response."),
                        address: format!("/response/{}", response.id),
                        recipient: profile.id.clone(),
                    },
                    None,
                )
                .await
            {
                continue;
            } else {
                // replace text with link to profile
                response.content = response.content.replace(
                    &format!("@{} ", mention),
                    &format!("[@{}](/+u/{}) ", profile.username, profile.id),
                )
            }
        }

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xresponses\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xresponses\" VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&response.author.id)
            .bind::<&String>(&response.question)
            .bind::<&String>(&response.content)
            .bind::<&String>(&response.id)
            .bind::<&String>(&response.timestamp.to_string())
            .bind::<&str>(&serde_json::to_string(&props.tags).unwrap_or("[]".to_string()))
            .bind::<&String>(&match serde_json::to_string(&response.context) {
                Ok(s) => s,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .bind::<&String>(&response.reply)
            .bind::<&String>(&response.edited.to_string())
            .bind::<i8>(0)
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                let tag = Database::anonymous_tag(&question.author.id);
                let mut is_allowed_to_receive_notif = true;

                if tag.0 {
                    // allow users who were just hiding their name to receive a notification
                    if let Ok(ua) = self.auth.get_profile(&tag.1).await {
                        is_allowed_to_receive_notif = true;
                        question.author = ua;
                    }
                }

                if (question.recipient.id != question.author.id) && is_allowed_to_receive_notif {
                    if let Err(_) = self
                        .auth
                        .create_notification(
                            NotificationCreate {
                                title: format!(
                                    "[@{}](/+u/{}) responded to a question you asked!",
                                    response.author.username, response.author.id
                                ),
                                content: format!(
                                    "{}: \"{}...\"",
                                    response.author.username,
                                    // we're only going to show 50 characters of the response in the notification
                                    response
                                        .content
                                        .clone()
                                        .chars()
                                        .take(50)
                                        .collect::<String>()
                                ),
                                address: format!("/response/{}", response.id),
                                recipient: question.author.id,
                            },
                            None,
                        )
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // handle global questions
                if question.recipient.username == "@" {
                    // this is a global ask, we need to respond to it and then just move on

                    // bump question response count
                    self.base
                        .cache
                        .incr(format!(
                            "rbeam.app.question_response_count:{}",
                            response.question
                        ))
                        .await;

                    // bump response count
                    self.base
                        .cache
                        .incr(format!(
                            "rbeam.app.response_count:{}",
                            response.author.id.clone()
                        ))
                        .await;

                    self.auth
                        .update_profile_response_count(
                            &response.author.id,
                            response.author.response_count + 1,
                        )
                        .await
                        .unwrap();

                    return Ok(response);
                } else {
                    // change recipient so it doesn't show up in inbox
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "UPDATE \"xquestions\" SET \"recipient\" = ? WHERE \"id\" = ?"
                        } else {
                            "UPDATE \"xquestions\" SET (\"recipient\") = ($1) WHERE \"id\" = $2"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&format!("ANSWERED:{}", question.recipient.id))
                        .bind::<&String>(&response.question)
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    }
                }

                // bump response count
                self.base
                    .cache
                    .incr(format!("rbeam.app.response_count:{}", response.author.id))
                    .await;

                // give us 2 coins :)
                if let Err(_) = self.auth.update_profile_coins(&response.author.id, 2).await {
                    return Err(DatabaseError::Other);
                }

                // return
                Ok(response)
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing response's reaction count
    ///
    /// # Arguments
    /// * `id`
    /// * `count`
    pub async fn update_response_reaction_count(&self, id: String, count: usize) -> Result<()> {
        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"reaction_count\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"reaction_count\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&i64>(&(count as i64))
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cache
                    .remove(format!("rbeam.app.response:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing response's content
    ///
    /// # Arguments
    /// * `id`
    /// * `content`
    /// * `user` - the user doing this
    pub async fn update_response_content(
        &self,
        id: String,
        content: String,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response_short(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check content length
        if content.len() > 4096 {
            return Err(DatabaseError::ContentTooLong);
        }

        if content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != response.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                return Err(DatabaseError::NotAllowed);
            } else {
                if let Err(e) = self
                    .audit(
                        user.id,
                        format!(
                            "Edited a response: [{}](/response/{})",
                            response.id, response.id
                        ),
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"content\" = ?, \"edited\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"content\", \"edited\") = ($1, $2) WHERE \"id\" = $3"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&content)
            .bind::<&String>(&utility::unix_epoch_timestamp().to_string())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cache
                    .remove(format!("rbeam.app.response:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing response's tags
    ///
    /// # Arguments
    /// * `id`
    /// * `tags`
    /// * `user` - the user doing this
    pub async fn update_response_tags(
        &self,
        id: String,
        tags: Vec<String>,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response_short(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != response.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                return Err(DatabaseError::NotAllowed);
            } else if let Err(e) = self
                .audit(
                    user.id,
                    format!(
                        "Edited a response's tags: [{}](/response/{})",
                        response.id, response.id
                    ),
                )
                .await
            {
                return Err(e);
            }
        }

        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"tags\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"tags\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&match serde_json::to_string(&tags) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cache
                    .remove(format!("rbeam.app.response:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update the tags of multiple responses. **All responses MUST be created by the same user.**
    ///
    /// # Arguments
    /// * `ids`
    /// * `tags`
    /// * `user` - the user doing this
    pub async fn update_response_tags_multiple(
        &self,
        ids: Vec<String>,
        tags: Vec<String>,
        user: Box<Profile>,
    ) -> Result<()> {
        // verify permissions for all responses
        for id in &ids {
            let response = match self.get_response_short(id.clone()).await {
                Ok(q) => q,
                Err(e) => return Err(e),
            };

            // check user permissions
            if user.group == -1 {
                // group -1 (even if it exists) is for marking users as banned
                return Err(DatabaseError::NotAllowed);
            }

            if user.id != response.author.id {
                // check permission
                let group = match self.auth.get_group_by_id(user.group).await {
                    Ok(g) => g,
                    Err(_) => return Err(DatabaseError::Other),
                };

                if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                    return Err(DatabaseError::NotAllowed);
                }
            }
        }

        // build sql
        let mut sql: String = String::new();

        for (i, id) in ids.clone().iter().enumerate() {
            let id = id.replace("'", "");
            if i == 0 {
                sql.push_str(&format!("\"id\" = '{id}'"));
            } else {
                sql.push_str(&format!(" OR \"id\" = '{id}'"));
            }
        }

        // update responses
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("UPDATE \"xresponses\" SET \"tags\" = ? WHERE {sql}")
        } else {
            format!("UPDATE \"xresponses\" SET (\"tags\") = ($1) WHERE {sql}")
        };

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&match serde_json::to_string(&tags) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .execute(c)
            .await
        {
            Ok(_) => {
                for id in ids {
                    self.base
                        .cache
                        .remove(format!("rbeam.app.response:{id}"))
                        .await;
                }

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete multiple responses. **All responses MUST be created by the same user.**
    ///
    /// # Arguments
    /// * `ids`
    /// * `user` - the user doing this
    pub async fn delete_response_multiple(
        &self,
        ids: Vec<String>,
        user: Box<Profile>,
    ) -> Result<()> {
        // verify permissions for all responses
        for id in &ids {
            let response = match self.get_response_short(id.clone()).await {
                Ok(q) => q,
                Err(e) => return Err(e),
            };

            // check user permissions
            if user.group == -1 {
                // group -1 (even if it exists) is for marking users as banned
                return Err(DatabaseError::NotAllowed);
            }

            if user.id != response.author.id {
                // check permission
                let group = match self.auth.get_group_by_id(user.group).await {
                    Ok(g) => g,
                    Err(_) => return Err(DatabaseError::Other),
                };

                if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                    return Err(DatabaseError::NotAllowed);
                }
            }
        }

        // build sql
        let mut sql: String = String::new();

        for (i, id) in ids.clone().iter().enumerate() {
            let id = id.replace("'", "");
            if i == 0 {
                sql.push_str(&format!("\"id\" = '{id}'"));
            } else {
                sql.push_str(&format!(" OR \"id\" = '{id}'"));
            }
        }

        // delete responses
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("DELETE FROM \"xresponses\" WHERE {sql}")
        } else {
            format!("DELTE FROM \"xresponses\" WHERE {sql}")
        };

        let c = &self.base.db.client;
        match sqlquery(&query).execute(c).await {
            Ok(_) => {
                for id in ids {
                    self.base
                        .cache
                        .remove(format!("rbeam.app.response:{id}"))
                        .await;
                }

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing response's context
    ///
    /// # Arguments
    /// * `id`
    /// * `context`
    /// * `user` - the user doing this
    pub async fn update_response_context(
        &self,
        id: String,
        context: ResponseContext,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response_short(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != response.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                return Err(DatabaseError::NotAllowed);
            } else if let Err(e) = self
                .audit(
                    user.id,
                    format!(
                        "Edited a response's context: [{}](/response/{})",
                        response.id, response.id
                    ),
                )
                .await
            {
                return Err(e);
            }
        }

        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"context\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"context\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&match serde_json::to_string(&context) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cache
                    .remove(format!("rbeam.app.response:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete an existing response
    ///
    /// Responses can only be deleted by their author.
    ///
    /// # Arguments
    /// * `id` - the ID of the response
    /// * `user` - the user doing this
    /// * `save_question` - if we should not delete the question too
    pub async fn delete_response(
        &self,
        id: String,
        user: Box<Profile>,
        save_question: bool,
    ) -> Result<()> {
        // make sure response exists
        let response = match self.get_response(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != response.1.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                return Err(DatabaseError::NotAllowed);
            } else {
                if let Err(e) = self
                    .audit(
                        user.id,
                        format!(
                            "Deleted a response by: [{}](/+u/{})",
                            response.1.author.id, response.1.author.id
                        ),
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xresponses\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xresponses\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cache
                    .remove(format!("rbeam.app.response:{}", id))
                    .await;

                // decr response count
                self.base
                    .cache
                    .decr(format!("rbeam.app.response_count:{}", response.1.author.id))
                    .await;

                // decr global question response count
                if response.0.recipient.username == "@" {
                    self.base
                        .cache
                        .decr(format!(
                            "rbeam.app.question_response_count:{}",
                            response.0.id
                        ))
                        .await;
                } else if !save_question {
                    // delete question
                    if let Err(e) = self
                        .delete_question(response.0.id, response.0.recipient)
                        .await
                    {
                        return Err(e);
                    };
                }

                // clear reactions
                if let Err(e) = self.clear_reactions(id).await {
                    return Err(e);
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Return a response's question to the inbox and delete the response
    ///
    /// # Arguments
    /// * `id`
    /// * `user` - the user doing this
    pub async fn unsend_response(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure the response exists
        let res = match self.get_response(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        let question = res.0;
        let response = res.1;

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != response.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_RESPONSES) {
                return Err(DatabaseError::NotAllowed);
            } else {
                if let Err(e) = self
                    .audit(
                        user.id.clone(),
                        format!(
                            "Unsent a response by: [{}](/+u/{})",
                            response.author.id, response.author.id
                        ),
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // update question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xquestions\" SET \"recipient\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xquestions\" SET (\"recipient\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&question.recipient.id)
            .bind::<&String>(&question.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                if let Err(e) = self.delete_response(response.id, user, true).await {
                    return Err(e);
                }

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // comments

    /// Get an existing comment
    ///
    /// # Arguments
    /// * `id`
    /// * `recurse` - should be FALSE when fetching counts to prevent a stack overflow
    #[async_recursion]
    pub async fn get_comment(
        &self,
        id: String,
        recurse: bool,
    ) -> Result<(ResponseComment, usize, usize)> {
        // check in cache
        match self
            .base
            .cache
            .get(format!("rbeam.app.comment:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<ResponseComment>(c.as_str()) {
                Ok(mut c) => {
                    c.author = match self.get_profile(c.author.id).await {
                        Ok(ua) => ua,
                        Err(e) => return Err(e),
                    };

                    return Ok((
                        c,
                        if recurse == true {
                            self.get_reply_count_by_comment(id.clone()).await
                        } else {
                            0
                        },
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }
                Err(_) => {
                    // bad cache entry, remove and continue
                    self.base
                        .cache
                        .remove(format!("rbeam.app.comment:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let reply = res.get("reply").unwrap().to_string();
        let comment = ResponseComment {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            response: res.get("response").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: from_row!(res->id()),
            timestamp: from_row!(res->timestamp(u128); 0),
            reply: if reply.is_empty() {
                None
            } else {
                match Box::pin(self.get_comment(reply, recurse)).await {
                    Ok(r) => Some(Box::new(r.0)),
                    Err(_) => None,
                }
            },
            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
            ip: from_row!(res->ip()),
            context: from_row!(res->context(json); DatabaseError::ValueError),
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cache
                .set(
                    format!("rbeam.app.comment:{}", id),
                    serde_json::to_string::<ResponseComment>(&comment).unwrap(),
                )
                .await;
        }

        // return
        Ok((
            comment,
            if recurse == true {
                self.get_reply_count_by_comment(id.clone()).await
            } else {
                0
            },
            self.get_reaction_count_by_asset(id).await,
        ))
    }

    /// Get all comments by their response ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_comments_by_response(
        &self,
        id: String,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"response\" LIKE ? AND \"reply\" = '' ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"response\" LIKE $1 AND \"reply\" = '' ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("{id}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all comments by their response ID, 12 at a time
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_comments_by_response_paginated(
        &self,
        id: String,
        page: i32,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = ? AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = $1 AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of comments by their response ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_comment_count_by_response(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.comment_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_comments_by_response(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(format!("rbeam.app.comment_count:{}", id), count.to_string())
            .await;

        count
    }

    /// Get all comments by their author ID
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_comments_by_author(
        &self,
        user: String,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all comments by their author ID (paginated by 25)
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    pub async fn get_comments_by_author_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all replies by their comment ID
    ///
    /// # Arguments
    /// * `id`
    /// * `recurse` - should be FALSE when fetching counts to prevent a stack overflow
    #[async_recursion]
    pub async fn get_replies_by_comment(
        &self,
        id: String,
        recurse: bool,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, recurse).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        if recurse == true {
                            self.get_reply_count_by_comment(id.clone()).await
                        } else {
                            0
                        },
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all replies by their comment ID, 12 at a time
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_replies_by_comment_paginated(
        &self,
        id: String,
        page: i32,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = from_row!(res->id());

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
                            ip: from_row!(res->ip()),
                            context: from_row!(res->context(json); DatabaseError::ValueError),
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of replies by their comment ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_reply_count_by_comment(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.reply_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_replies_by_comment(id.clone(), false)
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(format!("rbeam.app.reply_count:{}", id), count.to_string())
            .await;

        count
    }

    /// Create a new comment
    ///
    /// Comments can only be created by non-anonymous users.
    ///
    /// # Arguments
    /// * `props` - [`CommentCreate`]
    /// * `author` - the ID of the user creating the comment
    /// * `ip` - the IP address of the user creating the comment
    pub async fn create_comment(
        &self,
        props: CommentCreate,
        author: String,
        ip: String,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response_short(props.response.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check if the response author allows comments at all
        if response
            .author
            .metadata
            .is_true("rainbeam:disallow_response_comments")
        {
            return Err(DatabaseError::NotAllowed);
        }

        // check if we're posting this comment anonymously
        let tag = Database::anonymous_tag(&author);

        if tag.0 {
            // anonymous users cannot comment if disallowed by the response creator
            if response
                .author
                .metadata
                .is_true("sparkler:disallow_anonymous_comments")
            {
                return Err(DatabaseError::NotAllowed);
            }
        }

        if self
            .auth
            .get_ipblock_by_ip(&ip, &response.author.id)
            .await
            .is_ok()
        {
            return Err(DatabaseError::Blocked);
        }

        // check content length
        if props.content.len() > (64 * 32) {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check author permissions
        let author = match self.auth.get_profile(&author).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check relationship
        let relationship = self
            .auth
            .get_user_relationship(&response.author.id, &author.id)
            .await;

        if relationship.0 == RelationshipStatus::Blocked {
            return Err(DatabaseError::NotAllowed);
        }

        // check content length
        if props.content.trim().len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        if author.tier >= self.config.tiers.double_limits {
            if props.content.len() > (64 * 64) {
                return Err(DatabaseError::ContentTooLong);
            }
        } else {
            if props.content.len() > (64 * 32) {
                return Err(DatabaseError::ContentTooLong);
            }
        }

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let timestamp = utility::unix_epoch_timestamp();
        let mut comment = ResponseComment {
            author: author.clone(),
            response: response.id.clone(),
            content: props.content.trim().to_string(),
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            timestamp,
            reply: None,
            edited: timestamp,
            ip,
            context: CommentContext::default(),
        };

        // parse mentions
        for mention in Database::parse_mentions(comment.content.clone()) {
            let profile = match self.auth.get_profile(&mention).await {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Err(_) = self
                .auth
                .create_notification(
                    NotificationCreate {
                        title: format!(
                            "[@{}]({}) mentioned you in a comment!",
                            author.username, author.id
                        ),
                        content: format!("You were mentioned in a comment."),
                        address: format!("/comment/{}", comment.id),
                        recipient: profile.id.clone(),
                    },
                    None,
                )
                .await
            {
                continue;
            } else {
                // replace text with link to profile
                comment.content = comment.content.replace(
                    &format!("@{} ", mention),
                    &format!("[@{}](/+u/{}) ", profile.username, profile.id),
                )
            }
        }

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xcomments\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xcomments\" VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&comment.author.id)
            .bind::<&String>(&comment.response)
            .bind::<&String>(&comment.content)
            .bind::<&String>(&comment.id)
            .bind::<&String>(&comment.timestamp.to_string())
            .bind::<&String>(&props.reply)
            .bind::<&String>(&comment.timestamp.to_string())
            .bind::<&String>(&comment.ip)
            .bind::<&String>(&serde_json::to_string(&comment.context).unwrap())
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                if !props.reply.is_empty() {
                    // send notification
                    let reply = match self.get_comment(props.reply.clone(), false).await {
                        Ok(r) => r.0,
                        Err(e) => return Err(e),
                    };

                    let author_tag = Database::anonymous_tag(&reply.author.username);

                    if (reply.author.id != comment.author.id) && !author_tag.0 {
                        if let Err(_) = self
                            .auth
                            .create_notification(
                                NotificationCreate {
                                    title: if !tag.0 {
                                        format!(
                                            "[@{}](/+u/{}) replied to a comment you created!",
                                            comment.author.username, comment.author.id
                                        )
                                    } else {
                                        "Somebody replied to a comment you created!".to_string()
                                    },
                                    content: format!(
                                        "{}: \"{}...\"",
                                        comment.author.username,
                                        // we're only going to show 50 characters of the response in the notification
                                        comment
                                            .content
                                            .clone()
                                            .chars()
                                            .take(50)
                                            .collect::<String>()
                                    ),
                                    address: format!("/comment/{}", comment.id),
                                    recipient: reply.author.id,
                                },
                                None,
                            )
                            .await
                        {
                            return Err(DatabaseError::Other);
                        };
                    }

                    // bump reply count
                    self.base
                        .cache
                        .incr(format!("rbeam.app.reply_count:{}", props.reply))
                        .await;
                } else if response.author.id != comment.author.id {
                    if let Err(_) = self
                        .auth
                        .create_notification(
                            NotificationCreate {
                                title: if !tag.0 {
                                    format!(
                                        "[@{}](/@{}) commented on a response you created!",
                                        comment.author.username, comment.author.username
                                    )
                                } else {
                                    "Somebody commented on a response you created!".to_string()
                                },
                                content: format!(
                                    "{}: \"{}...\"",
                                    comment.author.username,
                                    // we're only going to show 50 characters of the response in the notification
                                    comment.content.clone().chars().take(50).collect::<String>()
                                ),
                                address: format!("/comment/{}", comment.id),
                                recipient: response.author.id,
                            },
                            None,
                        )
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // bump comment count
                self.base
                    .cache
                    .incr(format!("rbeam.app.comment_count:{}", response.id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update an existing comment's content
    ///
    /// # Arguments
    /// * `id`
    /// * `content`
    /// * `user` - the user doing this
    pub async fn update_comment_content(
        &self,
        id: String,
        content: String,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure the response exists
        let comment = match self.get_comment(id.clone(), false).await {
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        // check content length
        if content.len() > 4096 {
            return Err(DatabaseError::ContentTooLong);
        }

        if content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != comment.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_REACTIONS) {
                return Err(DatabaseError::NotAllowed);
            } else {
                if let Err(e) = self
                    .audit(
                        user.id,
                        format!(
                            "Edited a comment: [{}](/comment/{})",
                            comment.id, comment.id
                        ),
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // update comment
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xcomments\" SET \"content\" = ?, \"edited\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xcomments\" SET (\"content\", \"edited\") = ($1, $2) WHERE \"id\" = $3"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&content)
            .bind::<&String>(&utility::unix_epoch_timestamp().to_string())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cache
                    .remove(format!("rbeam.app.comment:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete an existing comment
    ///
    /// Comments can only be deleted by their author.
    ///
    /// # Arguments
    /// * `id` - the ID of the comment
    /// * `user` - the user doing this
    pub async fn delete_comment(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure comment exists
        let comment = match self.get_comment(id.clone(), false).await {
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        let res = match self.get_response_short(comment.response.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != comment.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            // check if we're the response author
            if user.id != res.author.id {
                // check if we're helper
                if !group.permissions.check(FinePermission::MANAGE_COMMENTS) {
                    return Err(DatabaseError::NotAllowed);
                } else {
                    if let Err(e) = self
                        .audit(
                            user.id,
                            format!(
                                "Deleted a comment by: [{}](/+u/{})",
                                comment.author.id, comment.author.id
                            ),
                        )
                        .await
                    {
                        return Err(e);
                    }
                }
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete comment
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xcomments\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xcomments\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cache
                    .remove(format!("rbeam.app.comment:{}", id))
                    .await;

                // decr response count
                self.base
                    .cache
                    .decr(format!("rbeam.app.comment_count:{}", comment.response))
                    .await;

                // decr reply count
                if comment.reply.is_some() {
                    self.base
                        .cache
                        .incr(format!("rbeam.app.reply_count:{}", comment.id))
                        .await;
                }

                // clear reactions
                if let Err(e) = self.clear_reactions(id).await {
                    return Err(e);
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // reactions

    /// Get an existing reaction
    ///
    /// # Arguments
    /// * `user` - the ID of the user
    /// * `asset` - the ID of the asset
    pub async fn get_reaction(&self, user: String, asset: String) -> Result<Reaction> {
        // check in cache
        match self
            .base
            .cache
            .get(format!("rbeam.app.reaction:{}:{}", user, asset))
            .await
        {
            Some(c) => match serde_json::from_str::<Reaction>(c.as_str()) {
                Ok(c) => return Ok(c),
                Err(_) => {
                    // delete invalid cached reaction
                    if self
                        .base
                        .cache
                        .remove(format!("rbeam.app.reaction:{}:{}", user, asset))
                        .await
                        == false
                    {
                        return Err(DatabaseError::Other);
                    }
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xreactions\" WHERE \"user\" = ? AND \"asset\" = ?"
        } else {
            "SELECT * FROM \"xreactions\" WHERE \"user\" = $1 AND \"asset\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&asset)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let reaction = Reaction {
            user: match self.get_profile(res.get("user").unwrap().to_string()).await {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            asset: res.get("asset").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
        };

        // store in cache
        self.base
            .cache
            .set(
                format!("rbeam.app.reaction:{}:{}", user, asset),
                serde_json::to_string::<Reaction>(&reaction).unwrap(),
            )
            .await;

        // return
        Ok(reaction)
    }

    /// Get all reactions by their asset ID
    ///
    /// # Arguments
    /// * `asset`
    pub async fn get_reactions_by_asset(&self, id: String) -> Result<Vec<Reaction>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xreactions\" WHERE \"asset\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xreactions\" WHERE \"asset\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<Reaction> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(Reaction {
                        user: match self.get_profile(res.get("user").unwrap().to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        asset: res.get("asset").unwrap().to_string(),
                        timestamp: from_row!(res->timestamp(u128); 0),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of reactions by their asset ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_reaction_count_by_asset(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cache
            .get(format!("rbeam.app.reaction_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_reactions_by_asset(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cache
            .set(
                format!("rbeam.app.reaction_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new reaction
    ///
    /// Reactions can only be created by non-anonymous users.
    ///
    /// # Arguments
    /// * `id` - the ID of the asset
    /// * `author` - the user creating the reaction
    pub async fn create_reaction(&self, id: String, author: Box<Profile>) -> Result<()> {
        let tag = Database::anonymous_tag(&author.username);

        if tag.0 {
            // anonymous users cannot comment
            return Err(DatabaseError::NotAllowed);
        }

        // make sure reaction doesn't already exist
        if let Ok(_) = self.get_reaction(author.id.clone(), id.clone()).await {
            return Err(DatabaseError::NotAllowed);
        }

        // check author permissions
        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        let reaction = Reaction {
            user: author,
            asset: id,
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xreactions\" VALUES (?, ?, ?)"
        } else {
            "INSERT INTO \"xreactions\" VALUES ($1, $2, $3)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&reaction.user.id)
            .bind::<&String>(&reaction.asset)
            .bind::<&String>(&reaction.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // bump reaction count
                self.base
                    .cache
                    .incr(format!("rbeam.app.reaction_count:{}", reaction.asset))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing reaction
    ///
    /// Reactions can only be deleted by their author.
    ///
    /// # Arguments
    /// * `id` - the ID of the reaction
    /// * `user` - the user doing this
    pub async fn delete_reaction(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure reaction exists
        let reaction = match self.get_reaction(user.id.clone(), id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != reaction.user.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_REACTIONS) {
                return Err(DatabaseError::NotAllowed);
            } else {
                if let Err(e) = self
                    .audit(
                        user.id.clone(),
                        format!(
                            "Deleted a reaction by: [{}](/+u/{})",
                            reaction.user.id, reaction.user.id
                        ),
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete reaction
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xreactions\" WHERE \"user\" = ? AND \"asset\" = ?"
        } else {
            "DELETE FROM \"xreactions\" WHERE \"user\" = $1 AND \"asset\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&user.id)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cache
                    .remove(format!("rbeam.app.reaction:{}:{}", user.id, id))
                    .await;

                // decr response count
                self.base
                    .cache
                    .decr(format!("rbeam.app.reaction_count:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete all reactions by their asset
    ///
    /// # Arguments
    /// * `id` - the ID of the asset
    pub async fn clear_reactions(&self, id: String) -> Result<()> {
        // delete reactions
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xreactions\" WHERE \"asset\" = ?"
        } else {
            "DELETE FROM \"xreactions\" WHERE \"asset\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // clear reaction count
                self.base
                    .cache
                    .decr(format!("rbeam.app.reaction_count:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // discover

    /// Get the top reacted-to responses (from the `cutoff`).
    ///
    /// # Arguments
    /// * `cutoff`
    pub async fn get_top_reacted_responses(&self, cutoff: u128) -> Result<Vec<FullResponse>> {
        // attempt to fetch from cache
        if let Some(res) = self
            .base
            .cache
            .get_timed::<Vec<FullResponse>>("rbeam.app.discover:top_reacted".to_string())
            .await
        {
            return Ok(res.1);
        };

        // ...
        let time = rainbeam_shared::unix_epoch_timestamp() - cutoff;

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE CAST(\"timestamp\" AS INT) > {time} ORDER BY CAST(\"reaction_count\" AS INT) DESC LIMIT 25")
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE CAST(\"timestamp\" AS INT) > {time} ORDER BY CAST(\"reaction_count\" AS INT) DESC LIMIT 25")
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<FullResponse> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // store
        self.base
            .cache
            .set_timed::<Vec<FullResponse>>(
                "rbeam.app.discover:top_reacted".to_string(),
                res.clone(),
            )
            .await;

        // return
        Ok(res)
    }

    /// Get the top "askers" (people who ask questions).
    ///
    /// # Arguments
    /// * `cutoff`
    pub async fn get_top_askers(&self) -> Result<Vec<(usize, Box<Profile>)>> {
        // attempt to fetch from cache
        if let Some(res) = self
            .base
            .cache
            .get_timed::<Vec<(usize, Box<Profile>)>>("rbeam.app.discover:top_askers".to_string())
            .await
        {
            return Ok(res.1);
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xprofiles\" WHERE CAST(\"question_count\" AS INT) > 0 ORDER BY CAST(\"question_count\" AS INT) DESC LIMIT 100")
        } else {
            format!("SELECT * FROM \"xprofiles\" WHERE CAST(\"question_count\" AS INT) > 0 ORDER BY CAST(\"question_count\" AS INT) DESC LIMIT 100")
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(usize, Box<Profile>)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let p = match self.auth.gimme_profile(res).await {
                        Ok(r) => r,
                        Err(_) => return Err(DatabaseError::Other),
                    };

                    out.push((p.question_count, p));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // store
        self.base
            .cache
            .set_timed::<Vec<(usize, Box<Profile>)>>(
                "rbeam.app.discover:top_askers".to_string(),
                res.clone(),
            )
            .await;

        // return
        Ok(res)
    }

    /// Get the profiles with the most responses.
    ///
    /// # Arguments
    /// * `cutoff`
    pub async fn get_top_responders(&self) -> Result<Vec<(usize, Box<Profile>)>> {
        // attempt to fetch from cache
        if let Some(res) = self
            .base
            .cache
            .get_timed::<Vec<(usize, Box<Profile>)>>(
                "rbeam.app.discover:top_responders".to_string(),
            )
            .await
        {
            return Ok(res.1);
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xprofiles\" WHERE CAST(\"response_count\" AS INT) > 0 ORDER BY CAST(\"response_count\" AS INT) DESC LIMIT 100")
        } else {
            format!("SELECT * FROM \"xprofiles\" WHERE CAST(\"response_count\" AS INT) > 0 ORDER BY CAST(\"response_count\" AS INT) DESC LIMIT 100")
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(usize, Box<Profile>)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let p = match self.auth.gimme_profile(res).await {
                        Ok(r) => r,
                        Err(_) => return Err(DatabaseError::Other),
                    };

                    out.push((p.response_count, p));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // store
        self.base
            .cache
            .set_timed::<Vec<(usize, Box<Profile>)>>(
                "rbeam.app.discover:top_responders".to_string(),
                res.clone(),
            )
            .await;

        // return
        Ok(res)
    }
}
