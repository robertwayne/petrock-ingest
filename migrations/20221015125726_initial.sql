CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS players
(
    id uuid DEFAULT uuid_generate_v4 (),
    username VARCHAR(255) UNIQUE PRIMARY KEY NOT NULL,
    online BOOLEAN DEFAULT false NOT NULL,
    experience INT DEFAULT 0 NOT NULL,  
    rank INT NOT NULL,

    /* Meta Data */
    created_on TIMESTAMP DEFAULT now() NOT NULL,
    last_modified TIMESTAMP DEFAULT now() NOT NULL
);

CREATE TABLE IF NOT EXISTS leaderboards
(
    id uuid DEFAULT uuid_generate_v4(),
    player VARCHAR(255) UNIQUE NOT NULL,
    CONSTRAINT fk_player FOREIGN KEY (player) REFERENCES players (username),

    /* Experience Data */
    total_experience INT NOT NULL,
    daily_experience INT NOT NULL,
    weekly_experience INT NOT NULL,
    monthly_experience INT NOT NULL,

    /* Meta Data */
    created_on TIMESTAMP DEFAULT now() NOT NULL,
    last_modified TIMESTAMP DEFAULT now() NOT NULL
);

/* History represents a series of leaderboard experience data. Each row is constrained to a player and contains time and the experience earned that day.*/
CREATE TABLE IF NOT EXISTS history
(
    id uuid DEFAULT uuid_generate_v4(),
    player VARCHAR(255) NOT NULL,

    experience INT NOT NULL,

    /* Meta Data */
    created_on TIMESTAMP DEFAULT CURRENT_DATE NOT NULL,
    last_modified TIMESTAMP DEFAULT now() NOT NULL,

    /* Composite Primary Key */
    PRIMARY KEY (player, created_on)
);

CREATE TABLE IF NOT EXISTS changelog
(
    id uuid DEFAULT uuid_generate_v4(),
    date TIMESTAMPTZ DEFAULT now() NOT NULL,
    changes TEXT[] DEFAULT '{}' NOT NULL
);

CREATE OR REPLACE FUNCTION get_last_week_experience(username TEXT)
RETURNS TABLE(experience INT)
AS $BODY$
    SELECT COALESCE(SUM (h.experience), 0)
    FROM history h
    WHERE h.player = username
    AND h.created_on BETWEEN NOW()::DATE-EXTRACT(DOW FROM NOW())::INTEGER-7
    AND NOW()::DATE-EXTRACT(DOW FROM NOW())::INTEGER;
$BODY$
LANGUAGE 'sql';

CREATE OR REPLACE FUNCTION get_last_month_experience(username TEXT)
RETURNS TABLE(experience INT)
AS $BODY$
    SELECT COALESCE(SUM (h.experience), 0)
    FROM history h
    WHERE h.player = username
    AND h.created_on >= DATE_TRUNC('month', NOW()) - INTERVAL '1 month'
    AND h.created_on < DATE_TRUNC('month', NOW())
$BODY$
LANGUAGE 'sql';

CREATE OR REPLACE FUNCTION get_current_month_experience(username TEXT)
RETURNS TABLE(experience INT)
AS $BODY$
    SELECT COALESCE(SUM (h.experience), 0)
    FROM history h
    WHERE h.player = username
    AND h.created_on >= DATE_TRUNC('month', CURRENT_DATE)
$BODY$
LANGUAGE 'sql';

CREATE OR REPLACE FUNCTION get_current_week_experience(username TEXT)
RETURNS TABLE(experience INT)
AS $BODY$
    SELECT COALESCE(SUM (h.experience), 0)
    FROM history h
    WHERE h.player = username
    AND h.created_on >= DATE_TRUNC('week', CURRENT_DATE) - INTERVAL '1 day'
    AND h.created_on < NOW()
$BODY$
LANGUAGE 'sql';

CREATE OR REPLACE FUNCTION player_update()
RETURNS TRIGGER
LANGUAGE 'plpgsql'
AS $$
BEGIN
    IF OLD.online = NEW.online THEN
        NEW.last_modified = OLD.last_modified;
    ELSE
        NEW.last_modified = now();
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION history_update()
RETURNS TRIGGER
LANGUAGE 'plpgsql'
AS $$
BEGIN
    IF OLD.experience = NEW.experience THEN
        NEW.last_modified = OLD.last_modified;
    ELSE
        NEW.last_modified = now();
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS on_update_history ON "public"."history";
DROP TRIGGER IF EXISTS on_update_player ON "public"."players";

CREATE TRIGGER on_update_history
BEFORE INSERT OR UPDATE ON history
FOR EACH ROW
EXECUTE PROCEDURE history_update();

CREATE TRIGGER on_update_player
BEFORE INSERT OR UPDATE ON players
FOR EACH ROW
EXECUTE PROCEDURE player_update();