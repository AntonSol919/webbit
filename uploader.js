 const WEBBIT = (() => {

     if (window.location.search.indexOf("uploader") < 0) {
         return false;
     }
     const webbitScript = document.getElementById("webbitScript");
     document.head.removeChild(webbitScript);

     function elIdObj(el, intoObj = {}) {
         let id = el.attributes.id;
         intoObj[id && id.value] = el;
         [...el.children].forEach((e) => elIdObj(e, intoObj));
         return intoObj;
     }
     const range = document.createRange();

     const out_style = `
position: fixed;
left:0px;
bottom:0px;
display:flex;
flex-flow: column;
align-items: center;
justify-content: center;
width:100vw;
z-index: 2147483647;
border: 1px solid #d3d3d3;
padding: 10px;
background-color: #2196F3;
color: #fff;
`
     const form_style = `
display:flex;
flex-flow: row no-wrap;
align-items: center;
gap: 12px;
justify-content: center;
`

     const editor = `
<div comment="this div will be automatically dropped on upload" id="webbit" style="${out_style}" contentEditable="false">
<div style="${form_style}"> 

  <label>edit:</label> <input type="checkbox" id="webbitDesignMode" />
  <label>Destination:</label> <div contenteditable id="webbitPath"> ${window.location.pathname}</div>
  <button id="webbitUpload">Upload</button>
  <a href="/about.html">About</a>
</div>
  <div id="webbitErrors" style="  flex-basis: 100%;"></div>
</div>
`
     const fragment = range.createContextualFragment(editor).firstElementChild;

     const {
         webbitPath,
         webbitUpload,
         webbitErrors,
         webbitDesignMode
     } = elIdObj(fragment);

     webbitDesignMode.addEventListener("click", () => document.body.contentEditable = webbitDesignMode.checked + "");

     function serializeDocument() {
         // The server demands files to start with:
         const HTML_PREFIX = `<!DOCTYPE html><html xmlns="http://www.w3.org/1999/xhtml"><head>`;
         document.body.contentEditable = "false";
         document.body.removeChild(fragment);
         let string = new XMLSerializer().serializeToString(document).replace(/&amp;/g, "&");
         document.body.appendChild(fragment);
         string = string.substring(string.indexOf("<head>") + "<head>".length);
         return HTML_PREFIX + string;
     }
     webbitUpload.addEventListener("click", () => {

         let dest = window.location.origin + webbitPath.innerText;
         const page = serializeDocument();
         const body = new Blob([page], {
             type: "text/xml"
         });
         fetch(dest, {
                 body,
                 method: "POST"
             })
             .then(async (response) => {
                 if (!response.ok) {
                     let body = await response.text();
                     throw Error(`Server returned ${response.status}: ${response.statusText}`);
                 }
                 window.open(response.headers.get("location"), '_blank');
             })
             .catch(logerr);
     });

     function logerr(e) {
         webbitErrors.prepend(range.createContextualFragment(`<div>${e}</div>`).firstElementChild);
     }
     window.addEventListener("load", () => document.body.appendChild(fragment));
 })();