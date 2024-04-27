CREATE TABLE favorite_sounds (
    user_id BIGINT UNSIGNED NOT NULL,
    sound_id INT UNSIGNED NOT NULL,
    FOREIGN KEY (sound_id) REFERENCES `sounds`(`id`) ON DELETE CASCADE ON UPDATE CASCADE,
    PRIMARY KEY (user_id, sound_id)
);
