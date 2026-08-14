CREATE TABLE bmr_image_tags(
    -- Unique ID of a given tag
    id INTEGER PRIMARY KEY,
    -- Human readable (short) name of the tag
    name TEXT NOT NULL,
    -- Human readable description of the tag with additional information, for
    -- example.
    description TEXT DEFAULT NULL,

    -- Equal tag names should refer to identical tags, anything else is just
    -- confusing.
    CONSTRAINT bmr_image_tags_have_unique_names UNIQUE(name)
)
