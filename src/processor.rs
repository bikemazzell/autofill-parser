use crate::models::{RawRecord, UserOutput};
use crate::constants::EMAIL_REGEX;

pub fn choose_identifier(record: &RawRecord, emails: &[String]) -> Option<String> {
    if let Some(email) = emails.first() {
        return Some(email.clone());
    }
    if let Some(id_val) = record.get("identifier") {
        let trimmed = id_val.trim();
        if !trimmed.is_empty() && EMAIL_REGEX.is_match(trimmed) {
            return Some(trimmed.to_lowercase());
        }
    }
    let username_patterns = ["email", "user", "login", "name"];
    for pattern in username_patterns {
        for (key, val) in record {
            let key_lower = key.to_lowercase();
            if key_lower.contains(pattern) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_lowercase());
                }
            }
        }
    }
    for val in record.values() {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

pub fn merge_records(base_user_output: &mut UserOutput, new_data_record: &RawRecord) {
    for (key, value) in new_data_record {
        // ensure we only add to other_fields
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

    #[test]
    fn test_choose_identifier_login_username_email_fields() {
        let mut record: RawRecord = HashMap::new();
        record.insert("login-username".to_string(), "juanpablovillabonal@gmail.com".to_string());
        record.insert("login-username".to_string(), "XxJuanCocoteroxX".to_string());
        let emails = vec!["juanpablovillabonal@gmail.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("juanpablovillabonal@gmail.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_multiple_emails_and_non_emails() {
        let mut record: RawRecord = HashMap::new();
        record.insert("email".to_string(), "100081118282110@otpku.com".to_string());
        record.insert("primary_first_name".to_string(), "Louisa".to_string());
        record.insert("primary_last_name".to_string(), "Khovanski".to_string());
        let emails = vec!["100081118282110@otpku.com".to_string(), "100094306124698@otpku.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("100081118282110@otpku.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_and_email_fields() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "aswanth1032007".to_string());
        record.insert("email".to_string(), "kannanalavil@gmail.com".to_string());
        record.insert("email2".to_string(), "aswanthkrishna103@gmail.com".to_string());
        let emails = vec!["kannanalavil@gmail.com".to_string(), "aswanthkrishna103@gmail.com".to_string(), "aswanth1032007@gmail.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("kannanalavil@gmail.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_colon_key_email() {
        let mut record: RawRecord = HashMap::new();
        record.insert(":r1:".to_string(), "karenbasta@microsoft.com".to_string());
        let emails = vec!["karenbasta@microsoft.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("karenbasta@microsoft.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_multiple_identifier_fields_with_email() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "A.espinozatelco".to_string());
        record.insert("identifier2".to_string(), "bastiasignacio14@gmail.com".to_string());
        let emails = vec!["bastiasignacio14@gmail.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("bastiasignacio14@gmail.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_and_email_with_phone() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "085260603071".to_string());
        record.insert("email".to_string(), "kaisar.group@yahoo.com".to_string());
        let emails = vec!["kaisar.group@yahoo.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("kaisar.group@yahoo.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_identifier_multiple_emails() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "niral.shah.1656@gmail.com".to_string());
        record.insert("identifier2".to_string(), "shreyac.office0898".to_string());
        let emails = vec!["niral.shah.1656@gmail.com".to_string()];
        assert_eq!(choose_identifier(&record, &emails), Some("niral.shah.1656@gmail.com".to_string()));
    }

    #[test]
    fn test_choose_identifier_case_insensitive_matching() {
        let mut record: RawRecord = HashMap::new();
        record.insert("USERNAME".to_string(), "UpperCaseKey".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("uppercasekey".to_string()));
    }

    #[test]
    fn test_choose_identifier_substring_patterns() {
        let mut record: RawRecord = HashMap::new();
        record.insert("user_login_name".to_string(), "SubstringMatch".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("substringmatch".to_string()));
    }

    #[test]
    fn test_choose_identifier_pattern_priority() {
        let mut record: RawRecord = HashMap::new();
        record.insert("user_name".to_string(), "UserName".to_string());
        record.insert("email_address".to_string(), "EmailAddress".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("emailaddress".to_string()));
    }

    #[test]
    fn test_choose_identifier_special_chars_in_keys() {
        let mut record: RawRecord = HashMap::new();
        record.insert("login-user.name_field".to_string(), "SpecialChars".to_string());
        let emails = Vec::new();
        assert_eq!(choose_identifier(&record, &emails), Some("specialchars".to_string()));
    }
} 