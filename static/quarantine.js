import init, {
    lk_keygen,
    lk_key_encrypt,
    lk_key_decrypt,

    lk_datapoint,
    lk_linkpoint,
    lk_keypoint,

    lk_read,
    lk_write,

    b64,
    Link,
    get_consts
} from './linkspace/linkspace.js';

function elIdObj(el, intoObj = {}) {
    let id = el.attributes.id;
    intoObj[id && id.value] = el;
    [...el.children].forEach((e) => elIdObj(e, intoObj));
    return intoObj;
}

const range = document.createRange();

function newEl(el) {
    return range.createContextualFragment(el).firstElementChild;
}
const loc = window.location;
const params = new URLSearchParams(loc.search);
const linkp_hash = params.get('hash');
const back = params.get('back');
const quarantinePktsUrl = `${loc.origin}/blob?hash=${linkp_hash}`;
const vouchUrl = `${loc.origin}/vouch?hash=${linkp_hash}`;

async function load_pkts(response) {
    if (!response.ok) {
        return alert(`Error - ${response.status} - ${response.statusText}`);
    }
    let blob = await response.blob();
    let buf = await blob.arrayBuffer();
    let bytes = new Uint8Array(buf);
    window.linkspace = await init();
    const {
        PUBLIC
    } = get_consts();

    console.log(PUBLIC);
    let pkts = [];
    while (bytes.length != 0) {
        let [p, rest] = lk_read(bytes);
        bytes = rest;
        pkts.push(p);
    }
    if (b64(pkts[0].hash) != linkp_hash) {
        throw Error("(NoSec) Error - hash mismatch - ?");
    }
    console.log(pkts);
    return pkts;
}
let errors = newEl(`<div id="errors"></div>`);
document.body.prepend(errors);

function logerr(e) {
    errors.prepend(range.createContextualFragment(`<div>${e}</div>`).firstElementChild);
}

function display_pkts(pkts) {
    let linkpkt = pkts[0];
    let userCtr = newEl(`
<div>
<form id="signForm" action="if_you_see_this_on_the_network_you_are_compromised.html" method="POST"">
 <input type="button" id="generateKey" value="new"/>
 <input id="encryptedKey" name="username" type="text"/>
 <input id="password" name="password" type="password" />
 <input id="submitEl" type="submit" value="Vouch" />
</form>
</div>
`);

    let {
        signForm,
        encryptedKey,
        password,
        generateKey,
        submitEl,
    } = elIdObj(userCtr);
    let displayKeyGen = () => {
        if (encryptedKey.value != "") {
            submitEl.style = "display:block";
            generateKey.style = "display:none";
        } else {
            generateKey.style = "display:block";
            submitEl.style = "display:none";
        }
    }
    displayKeyGen();
    encryptedKey.addEventListener("input", displayKeyGen);
    generateKey.addEventListener("click", async () => {
        if (encryptedKey.value != "" && !confirm("destroy key?")) {
            return false;
        }
        if (password.innerHTML == "") {
            password.value = prompt("password");
        }
        let bytes = new TextEncoder().encode(password.value);
        let tmp = generateKey.value;
        generateKey.value = "⏳";
        await new Promise(resolve => setTimeout(resolve, 100));
        encryptedKey.value = lk_key_encrypt(lk_keygen(), bytes);
        generateKey.value = tmp;
        displayKeyGen();
        return false;
    });

    signForm.onsubmit = function(_el) {
        try {
            let key = lk_key_decrypt(encryptedKey.value, new TextEncoder().encode(password.value));

            let keyp = lk_keypoint(key, "", {
                ...linkpkt.obj, // Get an object with the common fields
                create: undefined // Don't copy the create stamp field
            });
            let el = newEl("<pre></pre>");
            el.textContent = keyp.toString();
            document.body.prepend(el);
            const formData = new FormData();
            fetch(vouchUrl, {
                body: lk_write(keyp),
                method: "POST"
            }).then(async (response) => {
                if (!response.ok) {
                    let body = await response.text();
                    throw Error(`Server returned ${response.status}: ${response.statusText} - ${body}`);
                }
                let new_loc = back ? back : response.headers.get("location");
                window.location = new_loc;
            }).catch(logerr);

        } catch (e) {
            logerr(e);
        }
        return false;

    };


    document.body.appendChild(userCtr);
    for (let pkt of pkts) {
        let el = newEl("<pre></pre>");
        el.textContent = pkt.toString();
        document.body.appendChild(el);
    }
}

async function run() {
    init();
    const response = await fetch(quarantinePktsUrl)
        .then(load_pkts)
        .then(display_pkts);
}

run().catch((e) => console.log(e, e.toString && e.toString(), e.toJSON && e.toJSON()));