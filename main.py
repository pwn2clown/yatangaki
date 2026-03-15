import asyncio
from urllib.parse import urljoin, urlparse
from playwright.async_api import async_playwright, Playwright

def load_wordlist(wl_path: str) -> list[str]:
    with open(wl_path) as wl:
        return [word for word in wl.read().split("\n") if word]

EXCLUDED_SCRIPT_DOMAINS = load_wordlist("./confs/script_domain_exclusion.txt")
KNOWN_LIBS = load_wordlist("./confs/known_libs.txt")

async def run(playwright: Playwright):
    chromium = playwright.chromium
    browser = await chromium.connect("ws://127.0.0.1:3000")
    ctx = await browser.new_context()
    await ctx.add_init_script(path="hooks.js")
    page = await ctx.new_page()
    page.on("response", handle_response)
    page.on("console", handle_console)
    
    #  iter targets
    await page.goto("https://REDACTED")
    await page.wait_for_timeout(3000)
    await browser.close()

async def handle_response(response):
    if "javascript" in (response.headers.get("content-type") or "").lower() and \
        response.status == 200:

        url = urlparse(response.url)
        if matches_wl(url.netloc, EXCLUDED_SCRIPT_DOMAINS) or \
            matches_wl(url.path, KNOWN_LIBS):
            print(f"[+] excluded file {response.url[:150]} (known lib)")
            return

        try:
            body = await response.body()
            if body:
                print(f"[+] JS captured : {response.url} → ({len(body):,} bytes)")
        except:
            pass

async def handle_console(msg):
    text = msg.text
    if "soDOMx-src" in text:
        print(f"\033[91m[+] Use of URL param -> {text}\033[0m")
    else:
        #print(f"[INFO] console => {text}")
        pass

def matches_wl(content: str, wordlist: list[str]) -> bool:
        for word in wordlist:
            if word in content:
                return True
        return False

async def main():
    async with async_playwright() as playwright:
        await run(playwright)

asyncio.run(main())