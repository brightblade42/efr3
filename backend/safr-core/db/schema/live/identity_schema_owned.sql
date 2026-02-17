--
-- PostgreSQL database dump
--


-- Dumped from database version 18.1-custom-block16
-- Dumped by pg_dump version 18.2

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: eyefr; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA eyefr;


--
-- Name: logs; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA logs;


--
-- Name: bulk_insert_profiles(jsonb); Type: FUNCTION; Schema: eyefr; Owner: -
--

CREATE FUNCTION eyefr.bulk_insert_profiles(records jsonb) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    CREATE TEMPORARY TABLE temp_profiles(LIKE eyefr.profiles INCLUDING ALL) ON COMMIT DROP;

    INSERT INTO temp_profiles(last_name, first_name, middle_name, ext_id, img_url, raw_data)
    SELECT
        r->>'last_name',
        r->>'first_name',
        r->>'middle_name', 
        r->>'ext_id',
        r->>'img_url',
        (r->>'raw_data')::jsonb
    FROM jsonb_array_elements(records) as r;

    INSERT INTO eyefr.profiles(last_name, first_name,middle_name, ext_id, img_url, raw_data)
    SELECT last_name, first_name,middle_name, ext_id, img_url, raw_data
    FROM temp_profiles
    ON CONFLICT (ext_id) DO UPDATE SET
        last_name = EXCLUDED.last_name,
        first_name = EXCLUDED.first_name,
        middle_name = EXCLUDED.middle_name,
        ext_id = EXCLUDED.ext_id,
        img_url = EXCLUDED.img_url,
        raw_data = EXCLUDED.raw_data;
END;
$$;


--
-- Name: delete_identity_on_profile_delete(); Type: FUNCTION; Schema: eyefr; Owner: -
--

CREATE FUNCTION eyefr.delete_identity_on_profile_delete() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    AS $$
BEGIN
delete from public.identities where external_id = OLD.ext_id;
delete from eyefr.images where ext_id = OLD.ext_id;
RETURN OLD;
END
$$;


--
-- Name: search_profiles_by_name(text[], text[], text, integer, boolean); Type: FUNCTION; Schema: eyefr; Owner: -
--

CREATE FUNCTION eyefr.search_profiles_by_name(last_names text[] DEFAULT NULL::text[], first_names text[] DEFAULT NULL::text[], client_type text DEFAULT NULL::text, comp_id integer DEFAULT NULL::integer, must_have_image boolean DEFAULT false) RETURNS TABLE(last_name text, first_name text, ext_id text, img_url text)
    LANGUAGE sql STABLE
    AS $$
    select last_name, first_name,ext_id, img_url from eyefr.profiles p
    where (must_have_image is false or img_url != '')
    and (last_names is null or p.last_name ilike ANY(last_names))
    and (first_names is null or p.first_name ilike ANY(first_names))
    and (client_type is null or p.raw_data->>'type' = client_type)
    and (comp_id is null or (p.raw_data->'compId')::int = comp_id);
$$;


--
-- Name: update_profile_on_identity_insert(); Type: FUNCTION; Schema: eyefr; Owner: -
--

CREATE FUNCTION eyefr.update_profile_on_identity_insert() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
    begin
       update eyefr.profiles p set fr_id = NEW.public_id where p.ext_id = NEW.external_id;
       return new;
    end;
$$;


--
-- Name: log_enrollment_errors(text[], jsonb[]); Type: FUNCTION; Schema: logs; Owner: -
--

CREATE FUNCTION logs.log_enrollment_errors(p_code text[], p_payloads jsonb[]) RETURNS void
    LANGUAGE plpgsql
    AS $$
declare
  n_cat int := coalesce(array_length(p_code, 1), 0);
  n_pay int := coalesce(array_length(p_payloads, 1), 0);
begin
  if p_code is null or p_payloads is null then
    raise exception 'NULL array passed (cat=% pay=%)', p_code is null, p_payloads is null;
  end if;

  if n_cat = 0 or  n_pay = 0 then
    raise exception 'empty array passed (cat=% pay=%)', n_cat,  n_pay;
  end if;

  if n_cat <> n_pay then
    raise exception 'length mismatch (cat=% pay=%)', n_cat, n_pay;
  end if;

  insert into logs.enrollment (code, payload)
  select code, payload
  from unnest(p_code, p_payloads) as t(code, payload);
end;
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: camera; Type: TABLE; Schema: eyefr; Owner: -
--

CREATE TABLE eyefr.camera (
    id integer NOT NULL,
    name text NOT NULL,
    rtsp_url text,
    enabled boolean,
    user_name text,
    password text,
    direction integer,
    fr_stream_settings jsonb,
    min_match double precision,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: camera_id_seq; Type: SEQUENCE; Schema: eyefr; Owner: -
--

CREATE SEQUENCE eyefr.camera_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: camera_id_seq; Type: SEQUENCE OWNED BY; Schema: eyefr; Owner: -
--

ALTER SEQUENCE eyefr.camera_id_seq OWNED BY eyefr.camera.id;


--
-- Name: error_logs; Type: TABLE; Schema: eyefr; Owner: -
--

CREATE TABLE eyefr.error_logs (
    id integer NOT NULL,
    ext_id text,
    msg text,
    details jsonb,
    kind text,
    source text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: error_logs_id_seq; Type: SEQUENCE; Schema: eyefr; Owner: -
--

CREATE SEQUENCE eyefr.error_logs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: error_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: eyefr; Owner: -
--

ALTER SEQUENCE eyefr.error_logs_id_seq OWNED BY eyefr.error_logs.id;


--
-- Name: images; Type: TABLE; Schema: eyefr; Owner: -
--

CREATE UNLOGGED TABLE eyefr.images (
    id integer NOT NULL,
    ext_id text NOT NULL,
    data bytea NOT NULL,
    size real,
    url text,
    quality real DEFAULT 0.0 NOT NULL,
    acceptability real DEFAULT 0.0 NOT NULL,
    raw_data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: images_id_seq; Type: SEQUENCE; Schema: eyefr; Owner: -
--

CREATE UNLOGGED SEQUENCE eyefr.images_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: images_id_seq; Type: SEQUENCE OWNED BY; Schema: eyefr; Owner: -
--

ALTER SEQUENCE eyefr.images_id_seq OWNED BY eyefr.images.id;


--
-- Name: profiles; Type: TABLE; Schema: eyefr; Owner: -
--

CREATE TABLE eyefr.profiles (
    id integer NOT NULL,
    ext_id text NOT NULL,
    last_name text,
    first_name text,
    img_url text,
    raw_data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    fr_id text,
    middle_name text
);


--
-- Name: profiles_id_seq; Type: SEQUENCE; Schema: eyefr; Owner: -
--

CREATE SEQUENCE eyefr.profiles_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: profiles_id_seq; Type: SEQUENCE OWNED BY; Schema: eyefr; Owner: -
--

ALTER SEQUENCE eyefr.profiles_id_seq OWNED BY eyefr.profiles.id;


--
-- Name: registration_errors; Type: TABLE; Schema: eyefr; Owner: -
--

CREATE TABLE eyefr.registration_errors (
    ext_id text,
    fr_id text,
    message text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: enrollment; Type: TABLE; Schema: logs; Owner: -
--

CREATE TABLE logs.enrollment (
    id bigint NOT NULL,
    code text CONSTRAINT enrollment_category_not_null NOT NULL,
    payload jsonb NOT NULL,
    retry_count integer DEFAULT 0,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


--
-- Name: enrollment_id_seq; Type: SEQUENCE; Schema: logs; Owner: -
--

CREATE SEQUENCE logs.enrollment_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: enrollment_id_seq; Type: SEQUENCE OWNED BY; Schema: logs; Owner: -
--

ALTER SEQUENCE logs.enrollment_id_seq OWNED BY logs.enrollment.id;


--
-- Name: camera id; Type: DEFAULT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.camera ALTER COLUMN id SET DEFAULT nextval('eyefr.camera_id_seq'::regclass);


--
-- Name: error_logs id; Type: DEFAULT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.error_logs ALTER COLUMN id SET DEFAULT nextval('eyefr.error_logs_id_seq'::regclass);


--
-- Name: images id; Type: DEFAULT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.images ALTER COLUMN id SET DEFAULT nextval('eyefr.images_id_seq'::regclass);


--
-- Name: profiles id; Type: DEFAULT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.profiles ALTER COLUMN id SET DEFAULT nextval('eyefr.profiles_id_seq'::regclass);


--
-- Name: enrollment id; Type: DEFAULT; Schema: logs; Owner: -
--

ALTER TABLE ONLY logs.enrollment ALTER COLUMN id SET DEFAULT nextval('logs.enrollment_id_seq'::regclass);


--
-- Name: camera camera_name_key; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.camera
    ADD CONSTRAINT camera_name_key UNIQUE (name);


--
-- Name: camera camera_pkey; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.camera
    ADD CONSTRAINT camera_pkey PRIMARY KEY (id);


--
-- Name: error_logs error_logs_pkey; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.error_logs
    ADD CONSTRAINT error_logs_pkey PRIMARY KEY (id);


--
-- Name: images ext_id_ukey; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.images
    ADD CONSTRAINT ext_id_ukey UNIQUE (ext_id);


--
-- Name: CONSTRAINT ext_id_ukey ON images; Type: COMMENT; Schema: eyefr; Owner: -
--

COMMENT ON CONSTRAINT ext_id_ukey ON eyefr.images IS 'ext_id should be unique, one profile, one face. ';


--
-- Name: images images_ext_id_url_key; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.images
    ADD CONSTRAINT images_ext_id_url_key UNIQUE (ext_id, url);


--
-- Name: images images_pkey; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.images
    ADD CONSTRAINT images_pkey PRIMARY KEY (id);


--
-- Name: profiles profiles_ext_id_key; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.profiles
    ADD CONSTRAINT profiles_ext_id_key UNIQUE (ext_id);


--
-- Name: profiles profiles_pkey; Type: CONSTRAINT; Schema: eyefr; Owner: -
--

ALTER TABLE ONLY eyefr.profiles
    ADD CONSTRAINT profiles_pkey PRIMARY KEY (id);


--
-- Name: enrollment enrollment_pkey; Type: CONSTRAINT; Schema: logs; Owner: -
--

ALTER TABLE ONLY logs.enrollment
    ADD CONSTRAINT enrollment_pkey PRIMARY KEY (id);


--
-- Name: idx_errors_retry; Type: INDEX; Schema: logs; Owner: -
--

CREATE INDEX idx_errors_retry ON logs.enrollment USING btree (code, updated_at);


--
-- Name: profiles trg_delete_identity; Type: TRIGGER; Schema: eyefr; Owner: -
--

CREATE TRIGGER trg_delete_identity AFTER DELETE ON eyefr.profiles FOR EACH ROW EXECUTE FUNCTION eyefr.delete_identity_on_profile_delete();


--
-- PostgreSQL database dump complete
--


