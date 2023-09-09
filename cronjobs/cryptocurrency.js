const https = require("https");
const { exec } = require("child_process");
const fs = require("fs");
var urlParser = require("url");

const options = {
    hostname: "www.reddit.com",
    path: "/r/cryptocurrency/top.json?limit=1",
    headers: { "User-Agent": "Taggr.link" },
};

const stateFile = process.env.STATE_FILE;

https
    .get(options, (res) => {
        let body = "";

        res.on("data", (chunk) => {
            body += chunk;
        });

        res.on("end", () => {
            try {
                let json = JSON.parse(body);
                const { title, ups, num_comments, url, permalink } =
                    json.data.children[0].data;

                const prevPermalink = fs.readFileSync(stateFile, {
                    encoding: "utf8",
                    flag: "r",
                });

                if (prevPermalink == permalink) return;
                fs.writeFileSync(stateFile, permalink);

                const publisher = urlParser
                    .parse(url)
                    .hostname.replace("www.", "");
                let message =
                    `"${title}"${
                        publisher == "reddit.com"
                            ? ""
                            : ` [${publisher}](${url})`
                    }\\n\\n` +
                    `#CryptoCurrencySubreddit story: \`${ups}\` upvotes, [${num_comments} comments](https://reddit.com/${permalink})`;
                message = message.replaceAll("'", "'\"'\"'");
                const cmd = `dfx --identity icbot canister --network ic call 6qfxa-ryaaa-aaaai-qbhsq-cai add_post '("${message}", vec{})'`;
                exec(cmd, (error, stdout, stderr) => {
                    if (error) {
                        console.log(`error: ${error.message}`);
                        return;
                    }
                    if (stderr) {
                        console.log(`stderr: ${stderr}`);
                        return;
                    }
                    console.log(`Response: ${stdout}`);
                });
            } catch (error) {
                console.error(error.message);
            }
        });
    })
    .on("error", (error) => {
        console.error(error.message);
    });
