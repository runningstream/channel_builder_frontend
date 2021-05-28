table! {
    channel_list (id) {
        id -> Int4,
        userid -> Nullable<Int4>,
        name -> Varchar,
        data -> Varchar,
    }
}

table! {
    user_data (id) {
        id -> Int4,
        username -> Varchar,
        pass_hash -> Varchar,
        pass_hash_type -> Int4,
        validation_status -> Bool,
        validation_code -> Nullable<Varchar>,
        active_channel -> Nullable<Int4>,
    }
}

joinable!(channel_list -> user_data (userid));

allow_tables_to_appear_in_same_query!(
    channel_list,
    user_data,
);
