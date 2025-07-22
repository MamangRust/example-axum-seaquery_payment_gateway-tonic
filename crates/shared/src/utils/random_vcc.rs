use rand::Rng;

pub fn random_vcc() -> Result<String, &'static str> {
    let mut rng = rand::rng();

    let mut random_number = String::new();
    for _ in 0..15 {
        random_number.push_str(&rng.random_range(0..10).to_string());
    }

    let partial_card_number = format!("4{random_number}");
    let check_digit = calculate_check_digit(&partial_card_number);

    let credit_card_number = format!("{partial_card_number}{check_digit}");
    Ok(credit_card_number)
}

fn calculate_check_digit(number: &str) -> u32 {
    let mut sum = 0;
    let mut alternate = false;

    for digit_char in number.chars().rev() {
        let mut digit = digit_char.to_digit(10).expect("Invalid digit in number");

        if alternate {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }

        sum += digit;
        alternate = !alternate;
    }

    (10 - (sum % 10)) % 10
}
