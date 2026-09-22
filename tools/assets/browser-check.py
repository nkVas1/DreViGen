"""Optional visual delivery check; requires Playwright and an installed Chromium browser."""
from pathlib import Path
import json
import sys
from playwright.sync_api import sync_playwright

sys.stdout.reconfigure(encoding="utf-8")
root = Path(__file__).resolve().parents[2]
review = root / ".design-scratch/review"
review.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    chrome = Path(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
    browser = p.chromium.launch(executable_path=str(chrome) if chrome.exists() else None, headless=True)
    page = browser.new_page(viewport={"width": 1440, "height": 1080}, device_scale_factor=1)
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.goto((root / "assets/catalog.html").as_uri())
    page.evaluate("document.querySelectorAll('section').forEach(s=>s.style.contentVisibility='visible');document.querySelectorAll('img').forEach(i=>i.loading='eager')")
    page.wait_for_function('Array.from(document.querySelectorAll("img[src]")).every(i=>i.complete)')
    page.evaluate("window.scrollTo({top:0,behavior:'instant'})")
    page.screenshot(path=str(review / "catalog-desktop-final.png"))
    for group in ["10", "11", "12", "13"]:
        page.locator("#group-" + group).screenshot(path=str(review / f"group-{group}-final.png"))
    ids = page.locator("section").evaluate_all("els=>els.map(e=>e.id)")
    assert ids == [f"group-{i:02}" for i in range(1, 15)], ids
    broken = page.evaluate('Array.from(document.querySelectorAll("img[src]")).filter(i=>!i.naturalWidth).map(i=>i.src)')
    assert not broken, broken
    page.locator("input[type=search]").fill("Лавровый")
    assert page.locator(".asset:visible").count() == 1, "Search did not isolate one divider"
    page.locator("input[type=search]").fill("")
    page.locator("#group-10 [data-open]").first.click()
    assert page.locator("dialog").is_visible(), "Dialog did not open"
    page.keyboard.press("Escape")
    assert not page.locator("dialog").is_visible(), "Escape did not close dialog"
    page.get_by_role("button", name="Прозрачность", exact=True).click()
    assert page.locator("body").get_attribute("data-surface") == "checker"
    page.get_by_role("button", name="Вельма", exact=True).click()
    page.set_viewport_size({"width": 390, "height": 844})
    page.evaluate("window.scrollTo({top:0,behavior:'instant'})")
    page.screenshot(path=str(review / "catalog-mobile-final.png"))
    assert page.evaluate("document.documentElement.scrollWidth") == 390, "Mobile horizontal overflow"
    page.emulate_media(reduced_motion="reduce")
    assert page.evaluate("getComputedStyle(document.documentElement).scrollBehavior") == "auto"
    assert not errors, errors
    report = {
        "desktop": "1440x1080", "mobile": "390x844", "brokenImages": 0,
        "javascriptErrors": errors, "sectionOrder": "01-14", "search": "passed",
        "dialogAndEscape": "passed", "surfaceSwitch": "passed", "mobileOverflow": "none",
        "reducedMotion": "passed",
    }
    (root / "assets/browser-verification.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False))
    browser.close()
