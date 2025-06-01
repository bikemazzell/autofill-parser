use crate::models::{RawRecord, UserOutput};
use crate::constants::EMAIL_REGEX; // Import the regex
// HashMap import might have been removed by cargo fix, ensure it's here for tests.

/// Chooses a primary identifier for a user record.
/// Priority:
/// 1. Emails extracted by regex from any field value.
/// 2. Value of "identifier" key if it's a valid email.
/// 3. Value of "username" key (trimmed, lowercased).
/// 4. Value of "login" key (trimmed, lowercased).
pub fn choose_identifier(record: &RawRecord, emails: &[String]) -> Option<String> {
    if !emails.is_empty() {
        return Some(emails[0].clone());
    }

    // Check if the "identifier" key contains a valid email
    if let Some(id_value) = record.get("identifier") {
        let trimmed_id_value = id_value.trim();
        if EMAIL_REGEX.is_match(trimmed_id_value) {
            return Some(trimmed_id_value.to_lowercase().to_string());
        }
    }

    for key in ["username", "login"].iter() {
        if let Some(value) = record.get(*key) {
            let trimmed_value = value.trim();
            if !trimmed_value.is_empty() { 
                 return Some(trimmed_value.to_lowercase().to_string());
            }
        }
    }
    None
}

/// Merges a new record into an existing UserOutput record.
/// New keys from `new_data_record` are added if not present in `base_user_output.other_fields`.
/// The `identifier` and `emails` fields in `base_user_output` are not modified by this function.
pub fn merge_records(base_user_output: &mut UserOutput, new_data_record: &RawRecord) {
    for (key, value) in new_data_record {
        // Avoid overwriting special fields like 'identifier' or 'emails' if they somehow appear in new_data_record keys
        // and ensure we only add to other_fields.
        if key != "identifier" && key != "emails" { 
            base_user_output.other_fields.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap; // Make sure HashMap is in scope for tests

    #[test]
    fn test_choose_identifier_with_emails() {
        let record: RawRecord = HashMap::new();
        let emails = vec!["first@example.com".to_string(), "second@example.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("first@example.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_fallback_username() {
        let mut record: RawRecord = HashMap::new();
        record.insert("username".to_string(), " MyUser ".to_string()); // Value is .to_string()
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("myuser".to_string()));
    }

    #[test]
    fn test_choose_identifier_fallback_login() {
        let mut record: RawRecord = HashMap::new();
        record.insert("login".to_string(), "MyLogin".to_string()); // Value is .to_string()
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("mylogin".to_string()));
    }

    #[test]
    fn test_choose_identifier_fallback_preference() {
        let mut record: RawRecord = HashMap::new();
        record.insert("username".to_string(), "UserFirst".to_string());
        record.insert("login".to_string(), "LoginSecond".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("userfirst".to_string()));
    }
    
    #[test]
    fn test_choose_identifier_fallback_empty_username() {
        let mut record: RawRecord = HashMap::new();
        record.insert("username".to_string(), "  ".to_string()); 
        record.insert("login".to_string(), "some_login".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("some_login".to_string()));
    }

    #[test]
    fn test_choose_identifier_no_identifier() {
        let record: RawRecord = HashMap::new();
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), None);
    }

    #[test]
    fn test_choose_identifier_from_identifier_key_as_email() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), " EmailFromID@example.com ".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("emailfromid@example.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_key_not_an_email_fallback_username() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "not_an_email".to_string());
        record.insert("username".to_string(), " UserFromUsername ".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("userfromusername".to_string()));
    }

    #[test]
    fn test_choose_identifier_priority_emails_over_identifier_key() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "id_field_email@example.com".to_string());
        let emails_from_regex = vec!["regex_email@example.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails_from_regex), Some("regex_email@example.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_key_not_an_email_fallback_login() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "not_an_email_value".to_string());
        record.insert("login".to_string(), " UserFromLogin ".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("userfromlogin".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_key_empty_fallback_username() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "  ".to_string()); // Empty identifier value
        record.insert("username".to_string(), "UserFallback".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("userfallback".to_string()));
    }

    #[test]
    fn test_merge_records_simple_add() {
        let mut base = UserOutput {
            identifier: "id@example.com".to_string(),
            emails: vec!["id@example.com".to_string()],
            other_fields: HashMap::from([("key1".to_string(), "value1".to_string())]),
        };
        let new_data: RawRecord = HashMap::from([
            ("key2".to_string(), "value2".to_string()),
            ("key3".to_string(), "value3".to_string()),
        ]);
        merge_records(&mut base, &new_data);

        let mut expected_fields: RawRecord = HashMap::new();
        expected_fields.insert("key1".to_string(), "value1".to_string());
        expected_fields.insert("key2".to_string(), "value2".to_string());
        expected_fields.insert("key3".to_string(), "value3".to_string());
        assert_eq!(base.other_fields, expected_fields);
    }

    #[test]
    fn test_merge_records_no_overwrite() {
        let mut base = UserOutput {
            identifier: "id@example.com".to_string(),
            emails: vec!["id@example.com".to_string()],
            other_fields: HashMap::from([("key1".to_string(), "value1_base".to_string())]),
        };
        let new_data: RawRecord = HashMap::from([
            ("key1".to_string(), "value1_new".to_string()), 
            ("key2".to_string(), "value2_new".to_string()),
        ]);
        merge_records(&mut base, &new_data);

        let mut expected_fields: RawRecord = HashMap::new();
        expected_fields.insert("key1".to_string(), "value1_base".to_string());
        expected_fields.insert("key2".to_string(), "value2_new".to_string());
        assert_eq!(base.other_fields, expected_fields);
        assert_eq!(base.identifier, "id@example.com".to_string()); 
        assert_eq!(base.emails, vec!["id@example.com".to_string()]); 
    }

    #[test]
    fn test_merge_records_empty_new_data() {
        let mut base = UserOutput {
            identifier: "id@example.com".to_string(),
            emails: vec!["id@example.com".to_string()],
            other_fields: HashMap::from([("key1".to_string(), "value1".to_string())]),
        };
        let new_data: RawRecord = HashMap::new();
        let original_base_clone = base.clone();

        merge_records(&mut base, &new_data);
        assert_eq!(base, original_base_clone);
    }

    #[test]
    fn test_merge_records_empty_base_fields() {
        let mut base = UserOutput {
            identifier: "id@example.com".to_string(),
            emails: vec!["id@example.com".to_string()],
            other_fields: HashMap::new(), 
        };
        let new_data: RawRecord = HashMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        merge_records(&mut base, &new_data);

        assert_eq!(base.other_fields, new_data);
    }

    #[test]
    fn test_merge_records_new_data_has_special_keys() {
        let mut base = UserOutput {
            identifier: "base_id@example.com".to_string(),
            emails: vec!["base_id@example.com".to_string()],
            other_fields: HashMap::from([("key_a".to_string(), "val_a".to_string())]),
        };

        let mut new_data_with_special_keys: RawRecord = HashMap::new();
        new_data_with_special_keys.insert("identifier".to_string(), "new_id@example.com".to_string());
        new_data_with_special_keys.insert("emails".to_string(), "new_emails_val_SHOULD_NOT_BE_USED".to_string());
        new_data_with_special_keys.insert("key_b".to_string(), "val_b".to_string());

        merge_records(&mut base, &new_data_with_special_keys);

        let mut expected_fields: RawRecord = HashMap::new();
        expected_fields.insert("key_a".to_string(), "val_a".to_string());
        expected_fields.insert("key_b".to_string(), "val_b".to_string());

        assert_eq!(base.identifier, "base_id@example.com".to_string());
        assert_eq!(base.emails, vec!["base_id@example.com".to_string()]);
        assert_eq!(base.other_fields, expected_fields);
    }
} 