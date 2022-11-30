// Try to load existing invoice on page load.
async function init() {
    let response = await fetch("/update");
    if (response.status !== 410 ) {
        let invoiceUpdate = await response.json();
        await displayInvoiceUpdate(invoiceUpdate);
        await next(true);
    }
}
init()

async function next(hasAddress) {
    // Hide prep stuff, show payment stuff.
    document.getElementById("preperation-content").style.display = "None";
    document.getElementById("payment-content").style.display = "inherit";

    // Create invoice.
    if (!hasAddress) {
        document.getElementById("instruction").innerHTML = "Loading...";
        await newAddress();
    } else {
        await newWebsocket();
    }
}

async function newAddress() {
    const message = document.getElementById("message").value;
    const email = document.getElementById("email").value;
    let checkOutInfo = null;
    if (message != "" || email != "") {
        checkOutInfo = JSON.stringify({
            "email": email,
            "message": message
        })
    }
    const requestInfo = {
        method: "POST",
        body: checkOutInfo,
        headers: {
            'content-type': 'application/json'
        }
    }

    await fetch("/projects/acceptxmr/checkout", requestInfo);
    await newWebsocket();

    let response = await fetch("/update");
    let invoiceUpdate = await response.json();
    await displayInvoiceUpdate(invoiceUpdate);
}

async function displayInvoiceUpdate(invoiceUpdate) {
    console.log(invoiceUpdate);

    // Show paid/due.
    document.getElementById("paid").innerHTML = picoToXMR(invoiceUpdate.amount_paid);
    document.getElementById("due").innerHTML = picoToXMR(invoiceUpdate.amount_requested);

    // Show confirmations/required.
    var confirmations = invoiceUpdate.confirmations;
    document.getElementById("confirmations").innerHTML = Math.max(0, confirmations);
    document.getElementById("confirmations-required").innerHTML = invoiceUpdate.confirmations_required;

    // Show instructive text depending on invoice state.
    var instructionString = "Loading...";
    var instructionClass = "acceptxmr-instruction";
    var newAddressBtnHidden = true;
    var closeReason = null;
    if (confirmations != null && confirmations >= invoiceUpdate.confirmations_required) {
        instructionString = "Paid! Thank you";
        closeReason = "Confirmed";
    } else if (invoiceUpdate.amount_paid >= invoiceUpdate.amount_requested) {
        instructionString = "Paid! Waiting for Confirmation...";
    } else if (invoiceUpdate.expiration_in > 2) {
        instructionString = "Send Monero to Address Below";
    } else if (invoiceUpdate.expiration_in > 0) {
        instructionString = "Address Expiring Soon";
        instructionClass += " warning";
        newAddressBtnHidden = false;
    } else {
        instructionString = "Address Expired!";
        newAddressBtnHidden = false;
        closeReason = "Expired";
    }
    document.getElementById("instruction").innerHTML = instructionString;
    document.getElementById("instruction").classList = instructionClass;

    // Hide address if nearing expiration.
    document.getElementById("new-address-btn").hidden = newAddressBtnHidden;
    document.getElementById("address-copy-btn").disabled = !newAddressBtnHidden;
    if (newAddressBtnHidden) {
        document.getElementById("address").innerHTML = invoiceUpdate.address;

        if (!qrcode || typeof(qrcode) != "function") {
            // If qrcode isn't loaded yet, then wait.
            await new Promise(r => setTimeout(r, 100));
        }
        const qr = qrcode(0, "M");
        qr.addData(invoiceUpdate.uri);
        qr.make();
        document.getElementById('qrcode-container').innerHTML = qr.createSvgTag({ scalable: true });
    } else {
        document.getElementById("address").innerHTML = "Expiring or expired...";
        document.getElementById('qrcode-container').innerHTML = "<svg viewBox=\"0 0 100 100\" src=\"\"></svg>";
    }

    return closeReason;
}

async function newWebsocket() {
    // Close websocket if it already exists.
    if (typeof window.acceptxmrSocket != 'undefined') {
        window.acceptxmrSocket.close(1000, "New Address");

        await Promise.race([
            new Promise(r => setTimeout(r, 1000)),
            getPromiseFromEvent(window.acceptxmrSocket, "close")
        ]);
    }

    // Open websocket.
    connectWebsocket();
    document.getElementById("payment-pending-loader").style.display = "Block";

    await setWebsocketListeners();
}

function connectWebsocket() {
    if (location.protocol === 'https:') {
        var ws_protocol = 'wss:';
    } else {
        var ws_protocol = 'ws:';
    }
    window.acceptxmrSocket = new WebSocket(ws_protocol + "//" + window.location.host + "/projects/acceptxmr/ws/");
}

async function setWebsocketListeners() {
    window.acceptxmrSocket.onmessage = async function (event) {
        let closeReason = await displayInvoiceUpdate(JSON.parse(event.data));
        if (closeReason != null) {
            console.log(`Websocket closing: ${closeReason}`);
            window.acceptxmrSocket.close(1000, closeReason);
        }
    }

    // If the websocket closes cleanly, log it. Otherwise, alert the user.
    window.acceptxmrSocket.onclose = async function (event) {
        if (event.code === 1000) {
            console.log(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
        } else {
            // Try to reconnect once.
            try {
                connectWebsocket();
                while (window.acceptxmrSocket.readyState == 0) {
                    // If still connecting, wait 100 ms.
                    await new Promise(r => setTimeout(r, 100));
                }
                if (window.acceptxmrSocket.readyState == 1) {
                    await setWebsocketListeners();
                    console.log("Received abnormal close, but successfully restored connection.");
                    return
                }
            } catch (exception) {
                // Server process killed or network down.
                // Event.code is usually 1006 in this case.
                console.log(exception);
                alert('Connection died. If you have paid already, rest assured that it will still be processed.');
            }
        }
        document.getElementById("address").innerHTML = "";
        document.getElementById('qrcode-container').innerHTML = "<svg viewBox=\"0 0 100 100\" src=\"\"></svg>";
        document.getElementById("payment-pending-loader").style.display = "None";
    };

    window.acceptxmrSocket.onerror = function (error) {
        console.log(error);
    };
}

// Convert from piconeros to monero.
function picoToXMR(amount) {
    const divisor = 1_000_000_000_000;
    const xmr = Math.floor(amount / divisor) + amount % divisor / divisor;
    return new Intl.NumberFormat(undefined, { maximumSignificantDigits: 20 }).format(xmr);
}

// Make the copy button work.
function copyInvoiceAddress() {
    // Get the text field
    const copyText = document.getElementById("address");

    // Copy the text inside the text field
    navigator.clipboard.writeText(copyText.innerHTML);

    // Provide feedback
    document.getElementById("address-copy-btn").innerHTML = "Copied!";
    setTimeout(function () {
        document.getElementById("address-copy-btn").innerHTML = "Copy";
    }, 1000);
}

// A helper for waiting on events.
function getPromiseFromEvent(item, event) {
    return new Promise((resolve) => {
        const listener = () => {
            item.removeEventListener(event, listener);
            resolve();
        }
        item.addEventListener(event, listener);
    })
}
