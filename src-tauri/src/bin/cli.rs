use docforge::pdf_engine::*;
use std::env;
use std::fs;
use std::process::exit;

fn print_usage() {
    eprintln!(
        r#"
DocForge CLI - High-Performance Professional PDF Engine
===============================================================
Usage: docforge-cli <command> [arguments...]

Core Page Commands:
  info        <input.pdf>                              Show metadata, page count, and dimensions
  merge       <input1.pdf> <input2.pdf> ... -o <out>   Merge multiple PDFs into one
  delete-page <input.pdf> -p <page_1_based> -o <out>   Delete a specific page
  rotate      <input.pdf> -p <page> -d <90|180|270> -o <out>
                                                       Rotate a page by degrees
  reorder     <input.pdf> --from <p1> --to <p2> -o <out>
                                                       Move page position
  extract     <input.pdf> --pages <1,2,5-8> -o <out>   Extract specific pages
  duplicate   <input.pdf> -p <page> -o <out>           Duplicate a page
  create-blank -w <width> -h <height> -c <count> -o <out>
                                                       Create a blank PDF
  crop        <input.pdf> -p <page> -x <x> -y <y> -w <w> -h <h> -o <out>
                                                       Crop page bounding box

Text & Content Commands:
  add-text    <input.pdf> -p <page> -t <text> -x <x> -y <y> [--size 16] [--color #000000] -o <out>
                                                       Add text overlay to page
  extract-text <input.pdf> [-p <page>]                 Extract plain text from PDF
  search-text <input.pdf> -q <query>                   Search text occurrences across pages
  redact-text <input.pdf> -q <text_to_redact> -o <out> Search and deeply redact matching text

Security & Signatures:
  protect     <input.pdf> --password <pwd> -o <out>    Password-protect PDF
  unlock      <input.pdf> --password <pwd> -o <out>    Decrypt / remove PDF password
  verify-signatures <input.pdf>                        Verify digital signatures in PDF
  cert-list                                            List installed digital certificates

Annotations & Markup:
  watermark   <input.pdf> -t <text> [--opacity 0.3] [--size 48] -o <out>
                                                       Add diagonal watermark to all pages
  highlight   <input.pdf> -p <page> -x <x> -y <y> -w <w> -h <h> [--color #FFFF00] -o <out>
                                                       Add highlight annotation
  sticky-note <input.pdf> -p <page> -x <x> -y <y> -t <contents> -o <out>
                                                       Add sticky note comment

Forms & Fields (AcroForms):
  form-info   <input.pdf>                              List form fields and their current values
  add-form-field <input.pdf> -p <page> --type <text|checkbox|signature|dropdown> --name <name> -x <x> -y <y> -w <w> -h <h> -o <out>
                                                       Add interactive AcroForm field

Prepress & Conversion:
  preflight   <input.pdf>                              Run full PDF/X & prepress diagnostic
  ink-coverage <input.pdf> -p <page>                   Calculate CMYK ink coverage percentages
  convert-cmyk <input.pdf> -o <out>                    Convert RGB graphics to CMYK
  pdfa        <input.pdf> -o <out>                     Convert to archival PDF/A-1b format
  compress    <input.pdf> [--quality 1-100] -o <out>   Compress PDF stream objects
"#
    );
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

fn parse_pages(s: &str) -> Vec<usize> {
    let mut pages = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() == 2 {
                if let (Ok(start), Ok(end)) = (range[0].parse::<usize>(), range[1].parse::<usize>())
                {
                    for p in start..=end {
                        pages.push(p);
                    }
                }
            }
        } else if let Ok(p) = part.parse::<usize>() {
            pages.push(p);
        }
    }
    pages.sort();
    pages.dedup();
    pages
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return;
    }

    let command = &args[1];
    let subargs = &args[2..];

    match command.as_str() {
        "info" => {
            if subargs.is_empty() {
                eprintln!("Error: Missing input PDF. Usage: docforge-cli info <input.pdf>");
                exit(1);
            }
            let path = &subargs[0];
            let data = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Error reading file {path}: {e}");
                    exit(1);
                }
            };

            println!("File: {}", path);
            println!("Size: {} bytes", data.len());

            if let Ok(count) = get_page_count_from_data(&data) {
                println!("Pages: {}", count);
            }
            if let Ok(meta) = get_pdf_metadata(&data) {
                println!(
                    "Metadata: {}",
                    serde_json::to_string_pretty(&meta).unwrap_or_default()
                );
            }
            if let Ok(fields) = get_form_fields(&data) {
                println!("AcroForm Fields: {}", fields.len());
            }
            if let Ok(sigs) = verify_signature(&data, 0) {
                println!(
                    "Signatures: {}",
                    serde_json::to_string_pretty(&sigs).unwrap_or_default()
                );
            }
        }

        "merge" => {
            let mut inputs = Vec::new();
            let mut output = None;
            let mut i = 0;
            while i < subargs.len() {
                if subargs[i] == "-o" && i + 1 < subargs.len() {
                    output = Some(subargs[i + 1].clone());
                    i += 2;
                } else {
                    inputs.push(subargs[i].clone());
                    i += 1;
                }
            }

            if inputs.is_empty() || output.is_none() {
                eprintln!("Error: Missing inputs or output. Usage: docforge-cli merge <file1> <file2>... -o <out.pdf>");
                exit(1);
            }

            println!("Merging {} PDF files...", inputs.len());
            match merge_pdfs(&inputs) {
                Ok(merged) => {
                    let out_path = output.unwrap();
                    fs::write(&out_path, merged).unwrap();
                    println!("Successfully created {}", out_path);
                }
                Err(e) => {
                    eprintln!("Merge failed: {e}");
                    exit(1);
                }
            }
        }

        "delete-page" => {
            let input = &subargs[0];
            let page = get_arg(subargs, "-p")
                .and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(1);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            let zero_idx = if page > 0 { page - 1 } else { 0 };
            match delete_page(&data, zero_idx) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Page {page} deleted. Saved to {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "rotate" => {
            let input = &subargs[0];
            let page = get_arg(subargs, "-p")
                .and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(1);
            let deg = get_arg(subargs, "-d")
                .and_then(|d| d.parse::<i32>().ok())
                .unwrap_or(90);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            let zero_idx = if page > 0 { page - 1 } else { 0 };
            match rotate_page(&data, zero_idx, deg) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Page {page} rotated by {deg}°: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "extract" => {
            let input = &subargs[0];
            let pages_str = get_arg(subargs, "--pages").expect("--pages required (e.g. 1,2,5-7)");
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "extracted.pdf".into());

            let pages = parse_pages(&pages_str);
            let zero_based: Vec<usize> = pages
                .into_iter()
                .map(|p| if p > 0 { p - 1 } else { 0 })
                .collect();

            let data = fs::read(input).expect("Failed to read input");
            match extract_pages(&data, &zero_based) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Extracted pages saved to {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "create-blank" => {
            let w = get_arg(subargs, "-w")
                .and_then(|w| w.parse::<f64>().ok())
                .unwrap_or(595.0);
            let h = get_arg(subargs, "-h")
                .and_then(|h| h.parse::<f64>().ok())
                .unwrap_or(842.0);
            let c = get_arg(subargs, "-c")
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or(1);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "blank.pdf".into());

            match create_blank_pdf(w, h, c) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Created blank PDF ({w}x{h}, {c} page(s)): {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "add-text" => {
            let input = &subargs[0];
            let page = get_arg(subargs, "-p")
                .and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(1);
            let text = get_arg(subargs, "-t").expect("-t <text> required");
            let x = get_arg(subargs, "-x")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(50.0);
            let y = get_arg(subargs, "-y")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(500.0);
            let size = get_arg(subargs, "--size")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(14.0);
            let color = get_arg(subargs, "--color").unwrap_or_else(|| "#000000".into());
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            let zero_idx = if page > 0 { page - 1 } else { 0 };
            match add_text(&data, zero_idx, &text, x, y, size, &color) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Text added to page {page}: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "add-image" => {
            let input = &subargs[0];
            let page = get_arg(subargs, "-p")
                .and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(1);
            let img_path = get_arg(subargs, "--image").expect("--image <img_path> required");
            let x = get_arg(subargs, "-x")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(50.0);
            let y = get_arg(subargs, "-y")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(100.0);
            let w = get_arg(subargs, "-w")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(150.0);
            let h = get_arg(subargs, "-h")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(100.0);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            let img_data = fs::read(&img_path).expect("Failed to read image file");
            let zero_idx = if page > 0 { page - 1 } else { 0 };
            match add_image_to_page(&data, zero_idx, &img_data, x, y, w, h) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Image added to page {page}: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "add-header-footer" => {
            let input = &subargs[0];
            let header = get_arg(subargs, "--header").unwrap_or_default();
            let footer = get_arg(subargs, "--footer").unwrap_or_default();
            let size = get_arg(subargs, "--size")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(10.0);
            let margin = get_arg(subargs, "--margin")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(20.0);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match add_header_footer(&data, &header, &footer, size, margin) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Header/Footer added: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "add-page-numbers" => {
            let input = &subargs[0];
            let start_num = get_arg(subargs, "--start")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1);
            let pos = get_arg(subargs, "--pos").unwrap_or_else(|| "bottom-center".into());
            let size = get_arg(subargs, "--size")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(10.0);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "output.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match add_page_numbers(&data, &pos, size, start_num) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Page numbers added: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "extract-text" => {
            let input = &subargs[0];
            let data = fs::read(input).expect("Failed to read input");
            if let Some(page_str) = get_arg(subargs, "-p") {
                let page: usize = page_str.parse().unwrap_or(1);
                let zero_idx = if page > 0 { page - 1 } else { 0 };
                match get_page_text(&data, zero_idx) {
                    Ok(txt) => println!("{txt}"),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                let count = get_page_count_from_data(&data).unwrap_or(1);
                for i in 0..count {
                    println!("--- PAGE {} ---", i + 1);
                    if let Ok(txt) = get_page_text(&data, i) {
                        println!("{txt}");
                    }
                }
            }
        }

        "search-text" => {
            let input = &subargs[0];
            let query = get_arg(subargs, "-q").expect("-q <query> required");
            let data = fs::read(input).expect("Failed to read input");
            match search_text(&data, &query) {
                Ok(results) => {
                    println!("Found {} matching page(s) for '{}':", results.len(), query);
                    for r in results {
                        println!("  {}", serde_json::to_string(&r).unwrap_or_default());
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "redact-text" => {
            let input = &subargs[0];
            let query = get_arg(subargs, "-q").expect("-q <text_to_redact> required");
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "redacted.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match redact_text(&data, &query, "[REDACTED]") {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Redacted occurrences of '{}': saved to {}", query, out);
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "watermark" => {
            let input = &subargs[0];
            let text = get_arg(subargs, "-t").unwrap_or_else(|| "CONFIDENTIAL".into());
            let opacity = get_arg(subargs, "--opacity")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.3);
            let size = get_arg(subargs, "--size")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(48.0);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "watermarked.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match add_watermark(&data, &text, opacity, 45.0, size, "#888888", true, &[]) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Watermark '{text}' applied: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "protect" => {
            let input = &subargs[0];
            let pwd = get_arg(subargs, "--password").expect("--password <pwd> required");
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "protected.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match protect_pdf(&data, &pwd) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Protected PDF created: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "unlock" => {
            let input = &subargs[0];
            let pwd = get_arg(subargs, "--password").unwrap_or_default();
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "unlocked.pdf".into());

            let data = fs::read(input).expect("Failed to read input");
            match unlock_pdf(&data, &pwd) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("PDF unlocked: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "verify-signatures" => {
            let input = &subargs[0];
            let data = fs::read(input).expect("Failed to read input");
            match verify_signature(&data, 0) {
                Ok(sigs) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&sigs).unwrap_or_default()
                    );
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "preflight" => {
            let input = &subargs[0];
            let data = fs::read(input).expect("Failed to read input");
            match preflight_check(&data) {
                Ok(result) => {
                    println!(
                        "Preflight Status: {}",
                        if result.passed { "PASS" } else { "FAIL" }
                    );
                    println!("Score: {}/100", result.score);
                    println!("Summary: {} issues found", result.issues.len());
                    for issue in result.issues {
                        println!(
                            "  [{}] [{}] {}",
                            issue.severity, issue.category, issue.message
                        );
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "ink-coverage" => {
            let input = &subargs[0];
            let page = get_arg(subargs, "-p")
                .and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(1);
            let zero_idx = if page > 0 { page - 1 } else { 0 };

            let data = fs::read(input).expect("Failed to read input");
            match check_ink_coverage(&data, zero_idx) {
                Ok(coverage) => {
                    println!("CMYK Ink Coverage for Page {page}:");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&coverage).unwrap_or_default()
                    );
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "convert-cmyk" => {
            let input = &subargs[0];
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "cmyk.pdf".into());
            let data = fs::read(input).expect("Failed to read input");
            match convert_to_cmyk(&data) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Converted to CMYK: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "pdfa" => {
            let input = &subargs[0];
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "archival.pdf".into());
            let data = fs::read(input).expect("Failed to read input");
            match convert_to_pdfa(&data) {
                Ok(res) => {
                    fs::write(&out, res).unwrap();
                    println!("Converted to PDF/A: {out}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        "compress" => {
            let input = &subargs[0];
            let quality = get_arg(subargs, "--quality")
                .and_then(|q| q.parse::<u8>().ok())
                .unwrap_or(80);
            let out = get_arg(subargs, "-o").unwrap_or_else(|| "compressed.pdf".into());
            let data = fs::read(input).expect("Failed to read input");
            match compress_pdf_quality(&data, quality) {
                Ok(res) => {
                    let orig_len = data.len();
                    let new_len = res.len();
                    fs::write(&out, res).unwrap();
                    println!(
                        "Compressed from {} to {} bytes ({:.1}% of original): {}",
                        orig_len,
                        new_len,
                        (new_len as f64 / orig_len as f64) * 100.0,
                        out
                    );
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        other => {
            eprintln!("Unknown command: '{other}'");
            print_usage();
            exit(1);
        }
    }
}
