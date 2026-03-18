import asyncio
from urllib.parse import urlparse, urlencode, parse_qs, urlunparse
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
        if res is not None:
            log += f", res = {res}"
        
        color = Color.RED if "sink" in detection_type else Color.WARNING
        if "sink." in detection_type and not CANARY in value:
            return
        printc(color, log)

        if detection_type == "source.call" and name.split(".")[0] == "URLSearchParams":
            url_params.add(value[1:-1])
        elif detection_type == "source.manual-url-parse":
            url_params.add(res)

    async def handle_response(response):
        try:
            body = await response.body()
            body = body.decode()
            url = urlparse(response.url)
        except:
            #  If utf-8 decoding fails, we don't care about this resource
            return

        if "javascript" in (response.headers.get("content-type") or "").lower() and response.status == 200:
            if matches_wl(url.netloc, EXCLUDED_SCRIPT_DOMAINS) or \
                matches_wl(url.path, KNOWN_LIBS):
                print(f"[+] excluded file {response.url[:150]} (known lib)")
                return
            if body:
                printc(Color.OKBLUE, f"[+] JS captured : {response.url} → ({len(body):,} bytes)")

        elif url.netloc == target_domain:
            print(f"[+] other resource loaded {response.url}")
            query_string = parse_qs(url.query)
            for fetched_url_param in query_string.keys():
                print(f"[+] found query param {fetched_url_param}")
                url_params.add(fetched_url_param)

            if CANARY in body:
                print(f"[+] canary reflected in response ({response.url})")

    page = await ctx.new_page()
    await page.expose_function("__inspector_callback", js_callback_hook)
    page.on("response", handle_response)
    page.on("console", handle_console)
    
    #  iter targets
    base_url= f"https://REDACTED"
    target_url = f"{base_url}?id={CANARY}#{CANARY}"
    target_domain = urlparse(target_url).netloc
    await page.goto(target_url)
    await page.wait_for_timeout(5000)

    # reload with discovered params if any
    if url_params:
        print("-" * 100)
        print(f"[+] found used url params: {url_params}, reloading page.")

        query_dict = {p: CANARY for p in url_params}
        query_dict["hook"] = CANARY

        new_query = urlencode(query_dict, doseq=True)
        new_url = urlunparse((
            urlparse(base_url).scheme,
            urlparse(base_url).netloc,
            urlparse(base_url).path,
            "",
            new_query,
            f"{CANARY}"
        ))
        
        print(f"{new_url}")

        await page.goto(new_url)
        await page.wait_for_timeout(5000)
    else:
        print("[-] no discovered parameters, exiting...")

    print(f"[+] Analysis finished, found {len(url_params)} URL parameters")
    for p in url_params:
        print(f" - {p}")

    await browser.close()

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