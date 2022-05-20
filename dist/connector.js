const invoke = window.__TAURI__.invoke
const emit = window.__TAURI__.event.emit


$("#connect-btn").click(function() {
    var name = $("#name").val();
    var ipaddr = $("#ipaddr").val();

    emit("connect", { name: name, ipaddr: ipaddr });
});


$("#send-btn").click(function() {
    var msg_elem = $("#msg");
    var msg = msg_elem.val();
    msg_elem.val("");

    emit("send_msg", { msg: msg });
});