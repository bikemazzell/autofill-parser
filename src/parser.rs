use crate::constants::EMAIL_REGEX;
use crate::models::RawRecord;
use std::collections::{HashMap, HashSet};

pub fn parse_line(line: &str) -> RawRecord {
    if line.trim().is_empty() {
        return HashMap::new();
    }
    let mut record: RawRecord = HashMap::new();
    let pairs = line.split(',');
    for pair_str in pairs {
        let mut parts = pair_str.splitn(2, ':');
        if let Some(key) = parts.next() {
            let value = parts.next().unwrap_or("").trim();
            record.insert(key.trim().to_string(), value.to_string());
        }
    }
    record
}

pub fn extract_emails(record: &RawRecord) -> Vec<String> {
    let mut found_emails = Vec::new();
    let mut seen_emails = HashSet::new();
    let mut keys: Vec<_> = record.keys().cloned().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = record.get(&key) {
            for mat in EMAIL_REGEX.find_iter(value) {
                let email_str = mat.as_str().trim().to_lowercase();
                if !email_str.is_empty() && seen_emails.insert(email_str.clone()) {
                    found_emails.push(email_str);
                }
            }
        }
    }
    found_emails
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_line_simple() {
        let line = "key1:value1,key2:value2";
        let mut expected = HashMap::new();
        expected.insert("key1".to_string(), "value1".to_string());
        expected.insert("key2".to_string(), "value2".to_string());
        assert_eq!(parse_line(line), expected);
    }

    #[test]
    fn test_parse_line_empty_string() {
        assert_eq!(parse_line(""), HashMap::new());
        assert_eq!(parse_line("   "), HashMap::new());
    }

    #[test]
    fn test_parse_line_single_pair() {
        let mut expected: RawRecord = HashMap::new();
        expected.insert("key".to_string(), "value".to_string());
        assert_eq!(parse_line("key:value"), expected);
    }

    #[test]
    fn test_parse_line_multiple_pairs() {
        let mut expected: RawRecord = HashMap::new();
        expected.insert("key1".to_string(), "value1".to_string());
        expected.insert("key2".to_string(), "value2".to_string());
        assert_eq!(parse_line("key1:value1,key2:value2"), expected);
    }

    #[test]
    fn test_parse_line_with_spaces() {
        let mut expected: RawRecord = HashMap::new();
        expected.insert("key".to_string(), "value".to_string());
        assert_eq!(parse_line(" key : value "), expected);
    }

    #[test]
    fn test_parse_line_empty_value() {
        let mut expected: RawRecord = HashMap::new();
        expected.insert("key".to_string(), "".to_string());
        assert_eq!(parse_line("key:"), expected);
    }

    #[test]
    fn test_parse_line_empty_key_and_value() {
        let mut expected: RawRecord = HashMap::new();
        expected.insert("".to_string(), "".to_string());
        assert_eq!(parse_line(":"), expected);
    }

    #[test]
    fn test_parse_line_empty_and_invalid() {
        let mut record: RawRecord = HashMap::new();
        record.insert("key1".to_string(), "".to_string());
        record.insert("".to_string(), "value2".to_string());
        assert_eq!(parse_line("key1:,:value2"), record);
    }

    #[test]
    fn test_parse_line_duplicate_keys() {
        let line = "key1:value1,key2:value2,key1:value3";
        let record = parse_line(line);
        let mut expected: RawRecord = HashMap::new();
        expected.insert("key1".to_string(), "value3".to_string());
        expected.insert("key2".to_string(), "value2".to_string());
        assert_eq!(record, expected);
    }

    #[test]
    fn test_parse_line_handles_duplicate_identifier_correctly() {
        let line = "id_other:val,identifier:not_an_email,user:test,identifier:test@example.com,login:fallback";
        let record = parse_line(line);
        let mut expected: RawRecord = HashMap::new();
        expected.insert("id_other".to_string(), "val".to_string());
        expected.insert("identifier".to_string(), "test@example.com".to_string());
        expected.insert("user".to_string(), "test".to_string());
        expected.insert("login".to_string(), "fallback".to_string());
        assert_eq!(record, expected);
    }

    #[test]
    fn test_parse_line_handles_duplicate_email_key_correctly() {
        let line = "email:nota@real.email,email:actual_email@example.com,other:value";
        let record = parse_line(line);
        let mut expected: RawRecord = HashMap::new();
        expected.insert("email".to_string(), "actual_email@example.com".to_string());
        expected.insert("other".to_string(), "value".to_string());
        assert_eq!(record, expected);
    }

    #[test]
    fn test_extract_emails_no_emails() {
        let mut record: RawRecord = HashMap::new();
        record.insert("name".to_string(), "John Doe".to_string());
        record.insert("note".to_string(), "No email here".to_string());
        assert_eq!(extract_emails(&record), Vec::<String>::new());
    }

    #[test]
    fn test_extract_emails_single_email() {
        let mut record: RawRecord = HashMap::new();
        record.insert("email_field".to_string(), "test@example.com".to_string());
        assert_eq!(extract_emails(&record), vec!["test@example.com".to_string()]);
    }

    #[test]
    fn test_extract_emails_multiple_emails_in_one_value() {
        let mut record: RawRecord = HashMap::new();
        record.insert("contacts".to_string(), "Email: first@example.com, Second: second@example.com".to_string());
        let emails = extract_emails(&record);
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"first@example.com".to_string()));
        assert!(emails.contains(&"second@example.com".to_string()));
    }

    #[test]
    fn test_extract_emails_multiple_emails_in_different_values() {
        let mut record: RawRecord = HashMap::new();
        record.insert("email1".to_string(), "user1@example.com".to_string());
        record.insert("email2".to_string(), "user2@example.com".to_string());
        record.insert("desc".to_string(), " unrelated ".to_string());


        let emails = extract_emails(&record);
        let expected_emails = vec!["user1@example.com".to_string(), "user2@example.com".to_string()];
        assert_eq!(emails, expected_emails);
    }

    #[test]
    fn test_extract_emails_duplicate_emails_across_values() {
        let mut record: RawRecord = HashMap::new();
        record.insert("primary_email".to_string(), "main@example.com".to_string());
        record.insert("secondary_email".to_string(), "main@example.com".to_string());
        record.insert("cc_email".to_string(), "another@example.com, main@example.com".to_string());
        
        let emails = extract_emails(&record);
        
        let mut expected_emails = vec!["another@example.com".to_string(), "main@example.com".to_string()];
        let mut sorted_emails = emails.clone();
        sorted_emails.sort();
        expected_emails.sort();
        assert_eq!(sorted_emails, expected_emails);
        assert_eq!(emails.len(), 2);
    }

    #[test]
    fn test_extract_emails_case_insensitivity_and_trimming() {
        let mut record: RawRecord = HashMap::new();
        record.insert("contact_info".to_string(), "  TEST@EXAMPLE.COM  ".to_string());
        assert_eq!(extract_emails(&record), vec!["test@example.com".to_string()]);
    }
     #[test]
    fn test_extract_emails_mixed_validity() {
        let mut record: RawRecord = HashMap::new();
        record.insert("data".to_string(), "notanemail, test@example.com, another@place.org, invalid@, @invalid.com".to_string());
        let emails = extract_emails(&record);
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"test@example.com".to_string()));
        assert!(emails.contains(&"another@place.org".to_string()));
    }

    #[test]
    fn test_extract_emails_from_field_named_identifier_if_value_is_email() {
        let mut record: RawRecord = HashMap::new();
        record.insert("identifier".to_string(), "user_id_email@example.com".to_string());
        record.insert("other_field".to_string(), "some_value".to_string());
        
        let emails = extract_emails(&record);
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0], "user_id_email@example.com".to_string());
    }
} 