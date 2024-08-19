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
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_username" ON "xprofiles" ("username");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_follower" ON "xfollows" ("user");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_following" ON "xfollows" ("following");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_question" ON "xresponses" ("question");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_tokens" ON "xprofiles" ("tokens");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_warnings" ON "xwarnings" ("recipient");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_reactions" ON "xreactions" ("user");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_reactions_a" ON "xreactions" ("asset");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_memberships" ON "xcircle_memberships" ("circle");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_my_memberships" ON "xcircle_memberships" ("user");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_circles" ON "xcircles" ("id");
CREATE FULLTEXT INDEX IF NOT EXISTS "idx_circle_owner" ON "xcircles" ("owner");
