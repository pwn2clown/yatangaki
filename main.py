import asyncio
from urllib.parse import urljoin, urlparse
from playwright.async_api import async_playwright, Playwright

class Color:
    OKBLUE = "\033[94m"
    OKGREEN = "\033[92m"
    WARNING = "\033[93m"
    RED = "\033[91m"
    RST = "\033[0m"

def printc(color: Color, content: str):
    print(color + content + Color.RST)

def load_wordlist(wl_path: str) -> list[str]:
    with open(wl_path) as wl:
        return [word for word in wl.read().split("\n") if word]

EXCLUDED_SCRIPT_DOMAINS = load_wordlist("./confs/script_domain_exclusion.txt")
KNOWN_LIBS = load_wordlist("./confs/known_libs.txt")
CANARY = "pp_8963-632xdm"

async def run(playwright: Playwright):
    chromium = playwright.chromium
    browser = await chromium.connect("ws://127.0.0.1:3000")
    ctx = await browser.new_context()
    await ctx.add_init_script(f"window.__CANARY = '{CANARY}';")
    await ctx.add_init_script(path="hooks.js")
    #  scan data
    url_params = set()

    def js_callback_hook(data):
        detection_type = data['detection_type']
        value = data["value"]
        name = data["name"]
        res = data["result"]

        log = f"[+] client event [{detection_type}]: {name}, args = {value}";
        if res:
            log += f", res = {data['result']}"
        
        color = Color.RED
        if "sink" in detection_type:
            color = Color.WARNING
        printc(color, log)

        if detection_type == "source.call" and name.split(".")[0] == "URLSearchParams":
            url_params.add(value)

    page = await ctx.new_page()
    await page.expose_function("__inspector_callback", js_callback_hook)
    page.on("response", handle_response)
    page.on("console", handle_console)
    
    #  iter targets
    await page.goto(f"https://REDACTED/?hook={CANARY}#{CANARY}")
    await page.wait_for_timeout(5000)

    print(f"found used url params: {url_params}")
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
    if "__domscan" in text:
        print(text)

def matches_wl(content: str, wordlist: list[str]) -> bool:
        for word in wordlist:
            if word in content:
                return True
        return False

async def main():
    async with async_playwright() as playwright:
        await run(playwright)

asyncio.run(main())