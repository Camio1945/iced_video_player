//! JavaScript snippets, CSS overrides, and URL helpers for the Youdao
//! dictionary webview.

/// The darkreader.js library (embedded at compile time).
pub(super) const DARKREADER_JS: &str = include_str!("../../darkreader.js");

/// Aggressive dark CSS injected via DOM after page load (bypasses CSP).
/// Also calls DarkReader.enable() once the page DOM is fully populated.
/// Includes repeated removal of unwanted class nodes (top nav, ads).
pub(super) const DARK_OVERRIDE_JS: &str = r#"
(function(){
    var s=document.createElement('style');
    s.textContent=
        'html{background:#181818!important;color:#e0e0e0!important;}'+
        'body{background:#181818!important;color:#e0e0e0!important;}'+
        'div,section,article,header,footer,main,nav,aside,'+
        'ul,ol,li,p,h1,h2,h3,h4,h5,h6,a,span,b,i,em,strong,small,big,'+
        'table,thead,tbody,tr,td,th,dl,dt,dd,blockquote,pre,code,hr,br,'+
        'label,fieldset,legend,figure,figcaption,details,summary,address,'+
        '[class],[id],[role]{background-color:#181818!important;color:#e0e0e0!important;border-color:#333!important;}'+
        'input,button,textarea,select{background-color:#2a2a2a!important;color:#e0e0e0!important;border-color:#444!important;}'+
        'img,video,canvas,svg,picture,iframe{filter:invert(1) hue-rotate(180deg)!important;}';
    (document.head||document.documentElement).appendChild(s);
    document.documentElement.style.backgroundColor='#181818';
    document.documentElement.style.color='#e0e0e0';
    if(document.body){
        document.body.style.backgroundColor='#181818';
        document.body.style.color='#e0e0e0';
    }
    if(typeof DarkReader!=='undefined'){
        try{DarkReader.enable({brightness:100,contrast:90,sepia:10});}catch(e){}
    }

    // Remove unwanted elements (top nav, ads). Run three times to catch
    // elements that may appear at different times during page load.
    function removeUnwanted(){
        var classes=['content','m-top_vav','promo-ad'];
        for(var i=0;i<classes.length;i++){
            var els=document.getElementsByClassName(classes[i]);
            while(els.length>0){els[0].remove();}
        }
    }
    removeUnwanted();
    setTimeout(removeUnwanted,5000);
    setTimeout(removeUnwanted,20000);
})();
"#;

/// Mobile user-agent so Youdao serves the compact mobile page (better fit
/// for the narrow 360px sidebar).
pub(super) const MOBILE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

// ── URL encoding ─────────────────────────────────────────────────────────

/// Minimal percent-encoding for the word in the Youdao URL query.
pub(super) fn url_encode_word(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                let enc = c.encode_utf8(&mut buf);
                enc.bytes()
                    .map(|b| format!("%{:02X}", b))
                    .collect::<Vec<_>>()
                    .join("")
            }
        })
        .collect()
}

// ── JavaScript search injection ──────────────────────────────────────────

/// Build a JS snippet that fills the Youdao search box (`#search_input`)
/// with `word` and fires an Enter keypress, so the page's own search handler
/// performs the lookup — no page reload required.
///
/// The native `HTMLInputElement.prototype.value` setter is used (rather than
/// a direct `input.value = …`) so that framework-level getters/setters on the
/// input don't intercept the assignment. An `input` event is dispatched so the
/// framework syncs its internal model, then the full Enter key sequence
/// (`keydown` → `keypress` → `keyup`) is dispatched to trigger submission.
pub(super) fn search_script(word: &str) -> String {
    format!(
        r#"
(function(){{
    var input = document.getElementById('search_input');
    if(!input) return;
    var setter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype, 'value').set;
    if(setter){{ setter.call(input, {word:?}); }} else {{ input.value = {word:?}; }}
    input.dispatchEvent(new Event('input', {{bubbles:true}}));
    ['keydown','keypress','keyup'].forEach(function(t){{
        input.dispatchEvent(new KeyboardEvent(t, {{
            bubbles:true, cancelable:true,
            key:'Enter', code:'Enter',
            keyCode:13, which:13, charCode:13
        }}));
    }});
}})();
"#,
        word = word
    )
}
