// En ny signeringsnøkkel for manifestet (launcher.md 3.10 i det private repoet): kjøres av eierne på sin egen PC,
// aldri av GitHub. Trenger bare Node (18 eller nyere).
//
//   node utgivelse/ny-nokkel.mjs [fil]
//
// Den hemmelige delen skrives til filen (standard: signeringsnokkel.txt her), aldri på skjermen. Lim den inn som
// hemmeligheten KNOXLIFE_SIGNERINGSNOKKEL i miljøet «utgivelse» i GitHub, og slett filen. Den offentlige delen skrives
// ut: den er variabelen KNOXLIFE_MANIFEST_NOKKEL, som bygges inn i launcheren og ikke er hemmelig.
import { generateKeyPairSync } from "node:crypto";
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

const fil = resolve(process.argv[2] ?? "signeringsnokkel.txt");
const { privateKey, publicKey } = generateKeyPairSync("ed25519");
// De siste 32 bytene i PKCS#8 er nøkkelen selv (RFC 8410), og de siste 32 i SPKI er den offentlige.
const hemmelig = privateKey.export({ format: "der", type: "pkcs8" }).subarray(-32).toString("hex");
const offentlig = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
try {
  writeFileSync(fil, hemmelig + "\n", { mode: 0o600, flag: "wx" });
} catch (e) {
  console.error(e.code === "EEXIST" ? `${fil} finnes alt – flytt eller slett den først.` : String(e));
  process.exit(1);
}
console.log(`Den hemmelige nøkkelen står i ${fil}.`);
console.log("Lim den inn som KNOXLIFE_SIGNERINGSNOKKEL i miljøet «utgivelse» i GitHub, og slett filen etterpå.");
console.log("");
console.log(`Den offentlige nøkkelen (KNOXLIFE_MANIFEST_NOKKEL): ${offentlig}`);
