// Deterministic generator for products.json, the shared data set rendered by
// every benchmark app. Run once and commit the output:
//
//   node benchmarks/data/generate.mjs > benchmarks/data/products.json
//
// The checked-in JSON is the source of truth; regenerate only when the schema
// changes. All randomness flows from one seeded PRNG, so the output is stable
// across runs and machines.

const SEED = 0xc0ffee;
const PRODUCT_COUNT = 500;
const FEATURED_STRIDE = 42; // ids 1, 43, ..., 463 -> exactly 12 featured

function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rand = mulberry32(SEED);
const randInt = (min, max) => min + Math.floor(rand() * (max - min + 1));
const pick = (list) => list[Math.floor(rand() * list.length)];

const CATEGORIES = [
  { name: "Audio", slug: "audio" },
  { name: "Displays", slug: "displays" },
  { name: "Peripherals", slug: "peripherals" },
  { name: "Storage", slug: "storage" },
  { name: "Networking", slug: "networking" },
  { name: "Cameras", slug: "cameras" },
  { name: "Wearables", slug: "wearables" },
  { name: "Smart Home", slug: "smart-home" },
];

const BRANDS = [
  "Nimbus", "Vertex", "Aurora", "Zephyr", "Cascade", "Ember", "Quartz", "Onyx",
  "Halo", "Drift", "Pulse", "Summit", "Tundra", "Nova", "Cobalt", "Sable",
  "Meridian", "Frost", "Atlas", "Lumen", "Vector", "Orbit", "Prism", "Cinder",
  "Willow", "Falcon", "Granite", "Echo", "Slate", "Harbor",
];

const VARIANTS = ["", "", "", " Pro", " Mini", " Max", " Lite", " Ultra"];

const NOUNS = {
  audio: ["Wireless Headphones", "Studio Monitors", "Earbuds", "Soundbar", "Turntable", "Bookshelf Speakers"],
  displays: ["4K Monitor", "Ultrawide Display", "Portable Monitor", "E-Ink Display", "Gaming Monitor"],
  peripherals: ["Mechanical Keyboard", "Ergonomic Mouse", "Trackball", "Webcam", "USB Microphone", "Drawing Tablet"],
  storage: ["Portable SSD", "NVMe Drive", "External HDD", "MicroSD Card", "NAS Enclosure"],
  networking: ["Mesh Router", "Wi-Fi Extender", "Ethernet Switch", "Travel Router", "Access Point"],
  cameras: ["Action Camera", "Mirrorless Camera", "Dash Cam", "Security Camera", "Instant Camera"],
  wearables: ["Fitness Tracker", "Smartwatch", "Sleep Monitor", "Smart Ring", "Heart Rate Strap"],
  "smart-home": ["Smart Thermostat", "Video Doorbell", "Smart Plug", "Light Strip", "Robot Vacuum"],
};

// Each entry: [key, generator]. Generators return strings so no app has to
// format numbers beyond plain concatenation.
const SPEC_TEMPLATES = {
  audio: [
    ["Weight", () => `${randInt(140, 380)} g`],
    ["Battery life", () => `${randInt(18, 60)} hours`],
    ["Connectivity", () => pick(["Bluetooth 5.3", "Bluetooth 5.4", "2.4 GHz wireless", "USB-C", "3.5 mm analog"])],
    ["Driver size", () => `${randInt(28, 50)} mm`],
    ["Charging", () => pick(["USB-C", "Wireless", "USB-C + wireless", "Charging case"])],
    ["Warranty", () => `${randInt(1, 3)} years`],
  ],
  displays: [
    ["Panel size", () => `${randInt(24, 49)} in`],
    ["Resolution", () => pick(["1920 x 1080", "2560 x 1440", "3440 x 1440", "3840 x 2160", "5120 x 2160"])],
    ["Refresh rate", () => `${pick([60, 75, 120, 144, 165, 240])} Hz`],
    ["Panel type", () => pick(["IPS", "VA", "OLED", "Mini-LED", "E-Ink"])],
    ["Ports", () => pick(["2x HDMI, 1x DP", "1x HDMI, 2x DP", "HDMI + USB-C", "USB-C only"])],
    ["Warranty", () => `${randInt(1, 3)} years`],
  ],
  peripherals: [
    ["Weight", () => `${randInt(60, 1200)} g`],
    ["Connectivity", () => pick(["Bluetooth 5.3", "2.4 GHz wireless", "USB-C wired", "Wired + wireless"])],
    ["Battery life", () => pick(["N/A (wired)", `${randInt(40, 200)} hours`, `${randInt(1, 6)} months`])],
    ["Switch type", () => pick(["Linear", "Tactile", "Clicky", "Optical", "N/A"])],
    ["Compatibility", () => pick(["Windows, macOS, Linux", "Windows, macOS", "macOS, iPadOS", "Universal USB"])],
    ["Warranty", () => `${randInt(1, 3)} years`],
  ],
  storage: [
    ["Capacity", () => pick(["512 GB", "1 TB", "2 TB", "4 TB", "8 TB"])],
    ["Read speed", () => `${randInt(400, 7400)} MB/s`],
    ["Write speed", () => `${randInt(300, 6800)} MB/s`],
    ["Interface", () => pick(["USB-C 3.2", "USB4", "Thunderbolt 4", "PCIe 4.0", "PCIe 5.0", "SATA III"])],
    ["Endurance", () => `${randInt(150, 2400)} TBW`],
    ["Warranty", () => `${randInt(3, 5)} years`],
  ],
  networking: [
    ["Wireless standard", () => pick(["Wi-Fi 6", "Wi-Fi 6E", "Wi-Fi 7", "N/A (wired)"])],
    ["Max throughput", () => pick(["1.8 Gbps", "3.0 Gbps", "5.4 Gbps", "9.6 Gbps", "10 GbE"])],
    ["Ports", () => `${randInt(2, 8)}x RJ45`],
    ["Coverage", () => `${randInt(80, 550)} m2`],
    ["Mesh support", () => pick(["Yes", "No"])],
    ["Warranty", () => `${randInt(1, 3)} years`],
  ],
  cameras: [
    ["Sensor", () => pick(["1/2.3 in CMOS", "1 in CMOS", "APS-C", "Micro Four Thirds", "Full frame"])],
    ["Max video", () => pick(["1080p60", "4K30", "4K60", "5.3K60", "8K30"])],
    ["Weight", () => `${randInt(90, 700)} g`],
    ["Stabilization", () => pick(["Electronic", "Optical", "Hybrid", "In-body", "None"])],
    ["Water resistance", () => pick(["None", "IPX4", "IPX7", "10 m", "Housing required"])],
    ["Warranty", () => `${randInt(1, 2)} years`],
  ],
  wearables: [
    ["Weight", () => `${randInt(4, 85)} g`],
    ["Battery life", () => `${randInt(1, 21)} days`],
    ["Display", () => pick(["AMOLED", "Memory-in-pixel", "OLED", "None"])],
    ["Water resistance", () => pick(["3 ATM", "5 ATM", "10 ATM", "IP68"])],
    ["Sensors", () => pick(["HR, SpO2", "HR, SpO2, GPS", "HR, SpO2, GPS, ECG", "HR only"])],
    ["Warranty", () => `${randInt(1, 2)} years`],
  ],
  "smart-home": [
    ["Connectivity", () => pick(["Wi-Fi", "Wi-Fi + Thread", "Zigbee", "Matter over Wi-Fi", "Z-Wave"])],
    ["Power", () => pick(["Wired", "Battery", "Wired + battery backup", "USB-C"])],
    ["Assistant support", () => pick(["Alexa, Google", "Alexa, Google, HomeKit", "HomeKit only", "All major"])],
    ["Hub required", () => pick(["Yes", "No"])],
    ["Installation", () => pick(["Tool-free", "Screwdriver", "Professional recommended"])],
    ["Warranty", () => `${randInt(1, 3)} years`],
  ],
};

const DESCRIPTION_SENTENCES = [
  "The {name} was designed for people who notice the difference.",
  "Every part of the {name} earns its place, from the housing to the firmware.",
  "We rebuilt the internals of the {name} three times before shipping it.",
  "In daily use the {name} disappears into your routine, which is the point.",
  "The {name} pairs quickly, updates itself quietly, and stays out of your way.",
  "Materials were chosen for how they age: the {name} looks better after a year.",
  "Setup takes under five minutes, and the defaults are actually sensible.",
  "It holds up to travel, commutes, and the occasional drop off a desk.",
  "Battery projections are conservative; most owners report better numbers.",
  "The companion app is optional, not a requirement, and never nags.",
  "Repairability was a design goal: common parts are user-replaceable.",
  "Firmware updates have shipped monthly since launch, each with real fixes.",
  "Against others in the {category} category, it holds its own on every axis.",
  "Reviewers keep mentioning the finish, and in person it is even better.",
];

const REVIEW_AUTHORS_FIRST = [
  "Ada", "Bruno", "Carmen", "Devi", "Elias", "Freya", "Goran", "Hana",
  "Ivo", "Jules", "Kira", "Lars", "Mina", "Noel", "Odette", "Piotr",
  "Quinn", "Rosa", "Sven", "Tessa", "Umar", "Vera", "Wes", "Yara",
];

const REVIEW_AUTHORS_LAST = [
  "Fowler", "Okafor", "Lindqvist", "Marchetti", "Nakamura", "Petrov",
  "Silva", "Tanaka", "Urbina", "Vogel", "Whitfield", "Xu", "Yilmaz",
  "Zhang", "Andersen", "Baptiste", "Costa", "Duarte", "Eriksen", "Farah",
  "Gallo", "Haugen", "Iversen", "Janvier",
];

const REVIEW_TITLES = [
  "Great value", "Exceeded expectations", "Solid, with caveats", "Just buy it",
  "Better than the reviews say", "Good, not great", "Replaced my old one",
  "Impressive build quality", "Does what it promises", "Happy after two months",
  "A few rough edges", "My second one", "Gift that landed well", "Quietly excellent",
  "Almost perfect", "Would purchase again",
];

const REVIEW_SENTENCES = [
  "Delivery was fast and the packaging was compact and recyclable.",
  "It took a day to get used to, and now I cannot go back.",
  "The build quality is noticeably better than the price suggests.",
  "I compared three alternatives before settling on this one.",
  "Battery life matches the spec sheet almost exactly.",
  "Setup was painless and the instructions were actually readable.",
  "After six weeks of daily use there is no visible wear.",
  "Support answered my one question within a day.",
  "The finish picks up fingerprints, which is my only complaint.",
  "It is heavier than expected, but feels sturdier for it.",
  "Works flawlessly with my existing setup.",
  "I bought a second one for the office.",
  "Firmware update fixed the one quirk I had noticed.",
  "The price dropped a week after I bought it, still no regrets.",
];

const kebab = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");

function isoDate(startYear, dayRange) {
  const start = Date.UTC(startYear, 0, 1);
  const day = randInt(0, dayRange);
  return new Date(start + day * 86400000).toISOString().slice(0, 10);
}

const usedNames = new Set();

function productName(categorySlug) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const name = `${pick(BRANDS)} ${pick(NOUNS[categorySlug])}${pick(VARIANTS)}`;
    if (!usedNames.has(name)) {
      usedNames.add(name);
      return name;
    }
  }
  throw new Error("name space exhausted");
}

const products = [];

for (let id = 1; id <= PRODUCT_COUNT; id += 1) {
  const category = CATEGORIES[Math.floor(rand() * CATEGORIES.length)];
  const name = productName(category.slug);

  const specs = SPEC_TEMPLATES[category.slug].map(([key, gen]) => ({ key, value: gen() }));

  const paragraphCount = randInt(2, 3);
  const description = [];
  for (let p = 0; p < paragraphCount; p += 1) {
    const sentences = [];
    for (let s = 0, n = randInt(2, 3); s < n; s += 1) {
      sentences.push(
        pick(DESCRIPTION_SENTENCES).replaceAll("{name}", name).replaceAll("{category}", category.name),
      );
    }
    description.push(sentences.join(" "));
  }

  const reviews = [];
  for (let r = 0, n = randInt(5, 8); r < n; r += 1) {
    const body = [];
    for (let s = 0, m = randInt(1, 2); s < m; s += 1) {
      body.push(pick(REVIEW_SENTENCES));
    }
    reviews.push({
      author: `${pick(REVIEW_AUTHORS_FIRST)} ${pick(REVIEW_AUTHORS_LAST)}`,
      date: isoDate(2024, 880),
      rating_tenths: randInt(10, 50),
      title: pick(REVIEW_TITLES),
      body: body.join(" "),
    });
  }

  products.push({
    id,
    name,
    slug: `${kebab(name)}-${id}`,
    category: category.name,
    category_slug: category.slug,
    price_cents: randInt(999, 129999),
    rating_tenths: randInt(20, 50),
    review_count: reviews.length,
    featured: (id - 1) % FEATURED_STRIDE === 0 && id <= 1 + 11 * FEATURED_STRIDE,
    specs,
    description,
    reviews,
    related_ids: [], // filled below once every product has a category
  });
}

const idsByCategory = new Map(CATEGORIES.map((c) => [c.slug, []]));
for (const product of products) {
  idsByCategory.get(product.category_slug).push(product.id);
}

for (const product of products) {
  const pool = idsByCategory.get(product.category_slug).filter((id) => id !== product.id);
  const start = Math.floor(rand() * pool.length);
  product.related_ids = [0, 1, 2, 3].map((i) => pool[(start + i) % pool.length]);
}

process.stdout.write(`${JSON.stringify({ categories: CATEGORIES, products }, null, 2)}\n`);
