const invoke = window.__TAURI__.invoke
const emit = window.__TAURI__.event.emit
const listen = window.__TAURI__.event.listen


function sanitize(string) {
    const map = {
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#x27;',
        "/": '&#x2F;',
    };
    const reg = /[&<>"'/]/ig;
    return string.replace(reg, (match)=>(map[match]));
  }

$(document).ready(function() {
    var input = document.getElementById("msg");
    var ipaddr = document.getElementById("ipaddr");

    input.addEventListener("keypress", function(event) {
        if (event.key === "Enter") {
          event.preventDefault();
          $("#send-btn").click();
        }
    });

    ipaddr.addEventListener("keypress", function(event) {
        if (event.key === "Enter") {
          event.preventDefault();
          $("#connect-btn").click();
        }
    });

});


$("#connect-btn").click(function() {
    var name = $("#name").val();
    var ipaddr = $("#ipaddr").val();

    emit("connect", { name: name, ipaddr: ipaddr });
});


$("#send-btn").click(function() {
    var msg_elem = $("#msg");
    var msg = msg_elem.val();
    msg_elem.val("");
    if (msg.trim() != "") {
        emit("send_msg", { msg: msg });
    }
});


listen('add_to_chatbox', event => {
    var chatbox_elem = $("#chatbox");
    var payload = event.payload;
    var user = payload["user"];
    var message = "("+user+"): "+payload["message"]+"\n\n";
    chatbox_elem.append(sanitize(message));
    chatbox_elem.scrollTop(chatbox_elem[0].scrollHeight);
});


listen('add_info_to_chatbox', event => {
    var chatbox_elem = $("#chatbox");
    var payload = event.payload;
    var kind = payload["kind"];
    var message = kind+" "+payload["message"]+"\n\n";
    chatbox_elem.append(sanitize(message));
    chatbox_elem.scrollTop(chatbox_elem[0].scrollHeight);
});

