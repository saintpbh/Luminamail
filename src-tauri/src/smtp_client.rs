use lettre::{
    Message, SmtpTransport, Transport,
    message::{header::ContentType, MultiPart, SinglePart, Attachment},
    transport::smtp::authentication::Credentials,
};
use std::fs;

pub fn send_email(
    smtp_host: &str,
    smtp_port: u16,
    username: &str,
    password: &str,
    from_name: &str,
    from_email: &str,
    to_email: &str,
    cc_email: Option<&str>,
    bcc_email: Option<&str>,
    subject: &str,
    body_html: &str,
    signature_html: Option<&str>,
    attachments: Vec<String>,
) -> Result<(), String> {
    let full_html = if let Some(sig) = signature_html {
        format!("{}<br/><br/>{}", body_html, sig)
    } else {
        body_html.to_string()
    };

    let plain_text = html2text(&full_html);

    let mut builder = Message::builder()
        .from(format!("{} <{}>", from_name, from_email).parse().map_err(|e| format!("From 파싱 실패: {}", e))?);

    // Parse comma-separated To, Cc, Bcc
    for email in to_email.split(',') {
        let email = email.trim();
        if !email.is_empty() {
            builder = builder.to(email.parse().map_err(|e| format!("To 파싱 실패 ({}): {}", email, e))?);
        }
    }
    if let Some(cc) = cc_email {
        for email in cc.split(',') {
            let email = email.trim();
            if !email.is_empty() {
                builder = builder.cc(email.parse().map_err(|e| format!("Cc 파싱 실패 ({}): {}", email, e))?);
            }
        }
    }
    if let Some(bcc) = bcc_email {
        for email in bcc.split(',') {
            let email = email.trim();
            if !email.is_empty() {
                builder = builder.bcc(email.parse().map_err(|e| format!("Bcc 파싱 실패 ({}): {}", email, e))?);
            }
        }
    }

    let text_part = MultiPart::alternative()
        .singlepart(SinglePart::builder().header(ContentType::TEXT_PLAIN).body(plain_text))
        .singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(full_html));
    
    let mut mixed = MultiPart::mixed().multipart(text_part);

    for path_str in attachments {
        let path = std::path::Path::new(&path_str);
        if let Ok(file_body) = fs::read(path) {
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let content_type = mime_guess::from_path(path)
                .first_or_octet_stream()
                .as_ref()
                .parse()
                .unwrap_or_else(|_| ContentType::parse("application/octet-stream").unwrap());
            let attachment = Attachment::new(filename).body(file_body, content_type);
            mixed = mixed.singlepart(attachment);
        } else {
            // Alternatively, log error or return err. Let's return error so user knows attachment failed.
            return Err(format!("첨부파일 읽기 실패: {}", path_str));
        }
    }

    let email = builder
        .subject(subject)
        .multipart(mixed)
        .map_err(|e| format!("메일 생성 실패: {}", e))?;

    let creds = Credentials::new(username.to_string(), password.to_string());

    eprintln!("[SMTP] Connecting to {}:{} as {}", smtp_host, smtp_port, username);

    let mailer = if smtp_port == 465 {
        // Port 465: Direct SSL/TLS (implicit TLS)
        SmtpTransport::relay(smtp_host)
            .map_err(|e| format!("SMTP SSL 연결 실패: {}", e))?
            .port(smtp_port)
            .credentials(creds)
            .timeout(Some(std::time::Duration::from_secs(30)))
            .build()
    } else {
        // Port 587 or other: STARTTLS (explicit TLS upgrade)
        SmtpTransport::starttls_relay(smtp_host)
            .map_err(|e| format!("SMTP STARTTLS 연결 실패: {}", e))?
            .port(smtp_port)
            .credentials(creds)
            .timeout(Some(std::time::Duration::from_secs(30)))
            .build()
    };

    mailer.send(&email).map_err(|e| {
        eprintln!("[SMTP] Send failed: {:?}", e);
        format!("메일 전송 실패: {}", e)
    })?;
    eprintln!("[SMTP] Email sent successfully to {}", to_email);
    Ok(())
}

/// Simple HTML to plain text converter
fn html2text(html: &str) -> String {
    let re_tags = regex::Regex::new(r"(?is)<br\s*/?>").unwrap();
    let text = re_tags.replace_all(html, "\n");
    let re_all = regex::Regex::new(r"(?is)<.*?>").unwrap();
    let text = re_all.replace_all(&text, "");
    text.trim().to_string()
}
