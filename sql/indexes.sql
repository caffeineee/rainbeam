-- mariadb, postgresql, NO SQLITE
-- mysql remove "IF NOT EXISTS"
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_response" ON "xcomments" ("response");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_timestamp_r" ON "xresponses" ("timestamp");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_timestamp_q" ON "xquestions" ("timestamp");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_timestamp_c" ON "xcomments" ("timestamp");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_recipient_n" ON "xnotifications" ("recipient");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_recipient_q" ON "xquestions" ("recipient");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_author_r" ON "xresponses" ("author");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_author_q" ON "xquestions" ("author");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_author_c" ON "xcomments" ("author");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_username" ON "xprofiles" ("username");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_follower" ON "xfollows" ("user");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_following" ON "xfollows" ("following");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_question" ON "xresponses" ("question");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_tokens" ON "xprofiles" ("tokens");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_warnings" ON "xwarnings" ("recipient");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_reactions" ON "xreactions" ("user");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_reactions_a" ON "xreactions" ("asset");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_reactions_b" ON "xreactions" ("user", "asset");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_memberships" ON "xcircle_memberships" ("circle");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_my_memberships" ON "xcircle_memberships" ("user");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_circles" ON "xcircles" ("id");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_circle_owner" ON "xcircles" ("owner");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_context" ON "xresponses" ("context");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_tags" ON "xresponses" ("tags");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_uone" ON "xrelationships" ("one");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_utwo" ON "xrelationships" ("two");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_relationship" ON "xrelationships" ("one", "two");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_chats" ON "xchats" ("id");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_chat_members" ON "xchats" ("users");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_messages" ON "xmessages" ("chat");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_pages" ON "xpages" ("owner", "slug");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_ipblocks" ON "xipblocks" ("user", "ip");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_mail" ON "xmail" ("recipient");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_mail_author" ON "xmail" ("author");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_mail_id" ON "xmail" ("id");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_transactions_customer" ON "xugc_transactions" ("customer");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_transactions_customer_item" ON "xugc_transactions" ("customer", "item");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_item" ON "xugc_items" ("id");

CREATE FULLTEXT INDEX IF NOT EXISTS "idx_item_status" ON "xugc_items" ("status");
