import sys, os, requests, json

from functools import partial

from PyQt5 import QtCore, QtGui
from PyQt5.QtWidgets import QApplication,QScrollArea,QGridLayout, QMainWindow, QWidget
from PyQt5.QtWidgets import QPushButton, QPlainTextEdit, QTabWidget, QDialog, QVBoxLayout, QTableWidgetItem
from PyQt5.QtWidgets import QFormLayout, QLineEdit, QDialogButtonBox, QLabel, QHBoxLayout, QTableWidget
# from PyQt5.QtGui import QTextCursor, QTextDocument, QFontMetricsF

from pygments import highlight as _highlight
from pygments.lexers import SqlLexer
from pygments.formatters import HtmlFormatter

from AppKit import NSBundle
import keyring
from keyring.backends import macOS
keyring.set_keyring(macOS.Keyring())


def style() -> str:
    style = HtmlFormatter().get_style_defs('.highlight')
    return style


def highlight(text):
    # Generated HTML contains unnecessary newline at the end
    # before </pre> closing tag.
    # We need to remove that newline because it's screwing up
    # QTextEdit formatting and is being displayed
    # as a non-editable whitespace.
    highlighted_text: str = _highlight(text, SqlLexer(), HtmlFormatter()).strip()

    # Socrata Specific things
    highlighted_text = highlighted_text.replace("|&gt;", "<span style='color:blue'>|&gt;</span>")
    highlighted_text = highlighted_text.replace('(:</span><span class="n">id</span>', "(</span><span style='color:#006c82'>:id</span>")
    highlighted_text = highlighted_text.replace('(:</span><span class="n">created_at</span>', "(</span><span style='color:#006c82'>:created_at</span>")
    highlighted_text = highlighted_text.replace('(:</span><span class="n">updated_at</span>', "(</span><span style='color:#006c82'>:updated_at</span>")

    # Split generated HTML by last newline in it
    # argument 1 indicates that we only want to split the string
    # by one specified delimiter from the right.
    parts = highlighted_text.rsplit("\n", 1)

    # Glue back 2 split parts to get the HTML without last
    # unnecessary newline
    highlighted_text_no_last_newline = "".join(parts)
    return highlighted_text_no_last_newline


class ConnectionDialog(QDialog):
    """Dialog."""
    def __init__(self, parent=None):
        """Initializer."""
        super().__init__(parent)
        self.setWindowTitle('New Connection')
        dlgLayout = QVBoxLayout()
        formLayout = QFormLayout()
        self.domain = QLineEdit()
        self.domain.setObjectName("domain")
        self.domain.setText("")
        formLayout.addRow('domain:', self.domain)
        dlgLayout.addLayout(formLayout)
        btns = QDialogButtonBox()
        btns.setStandardButtons(
            QDialogButtonBox.Cancel | QDialogButtonBox.Ok)
        dlgLayout.addWidget(btns)
        self.setLayout(dlgLayout)
        btns.clicked.connect(self.close)

class QueryDialog(QDialog):
    """Dialog."""
    def __init__(self, parent=None):
        """Initializer."""
        super().__init__(parent)
        self.setWindowTitle('New Query')
        dlgLayout = QVBoxLayout()
        formLayout = QFormLayout()
        self.queryname = QLineEdit()
        self.queryname.setObjectName("queryname")
        self.queryname.setText("")
        formLayout.addRow('Query Name:', self.queryname)
        dlgLayout.addLayout(formLayout)
        btns = QDialogButtonBox()
        btns.setStandardButtons(
            QDialogButtonBox.Cancel | QDialogButtonBox.Ok)
        dlgLayout.addWidget(btns)
        self.setLayout(dlgLayout)
        btns.clicked.connect(self.close)


class Queries(QWidget):
    def __init__(self, queries = [], connection=""):
        self.connection = connection
        QWidget.__init__(self)
        layout = QVBoxLayout()
        self.setLayout(layout)
        self.tabwidget = QTabWidget()
        self.tabwidget.setTabsClosable(True)
        for query in queries:
            queryWidget = QWidget()
            queryLayout = QVBoxLayout()
            label0 = QLabel(connection)
            queryLayout.addWidget(label0)
            datasetid = QLineEdit()
            datasetid.setText(query['datasetid'])
            queryLayout.addWidget(datasetid)
            label1 = QPlainTextEdit()
            highlighted = highlight(query["query"])
            doc = label1.document()
            doc.setDefaultStyleSheet(style())
            label1.setDocument(doc)
            label1.appendHtml(highlighted)
            label1.textChanged.connect(partial(self.rehighlight, label1))
            label1.setTabStopDistance(
                QtGui.QFontMetricsF(label1.font()).horizontalAdvance(' ') * 8)
            queryLayout.addWidget(label1)
            queryWidget.setLayout(queryLayout)
            self.tabwidget.addTab(queryWidget, query["queryname"])
        self.tabwidget.tabCloseRequested.connect(lambda index: self.tabwidget.removeTab(index))
        layout.addWidget(self.tabwidget)
        buttonWidget = QWidget()
        buttonLayout = QHBoxLayout()
        self.saveButton = QPushButton("Save Query")
        buttonLayout.addWidget(self.saveButton)
        self.submitButton = QPushButton("Execute")
        buttonLayout.addWidget(self.submitButton)
        buttonWidget.setLayout(buttonLayout)
        layout.addWidget(buttonWidget)
        self.setMaximumHeight(500)
        self.changed = False
        self.highlighted = False

    def rehighlight(self, label: QPlainTextEdit):
        if not self.changed and not self.highlighted:
            self.changed = True
            current_cursor = label.textCursor()
            current_cursor_position = current_cursor.position()
            highlighted = highlight(label.toPlainText())
            label.clear()
            self.highlighted = True
            # Have to add newlines :(
            label.appendHtml(highlighted + "<div></div>")
            current_cursor.setPosition(current_cursor_position)
            label.setTextCursor(current_cursor)
        else:
            self.changed = False
            self.highlighted = False

    def addQuery(self, newQuery):
        queryWidget = QWidget()
        queryLayout = QVBoxLayout()
        label0 = QLabel(newQuery['domain'])
        queryLayout.addWidget(label0)

        datasetid = QLineEdit()
        datasetid.setText(newQuery['datasetid'])
        queryLayout.addWidget(datasetid)

        label1 = QPlainTextEdit()
        doc = label1.document()
        doc.setDefaultStyleSheet(style())
        highlighted = highlight(newQuery["query"])
        label1.setDocument(doc)
        label1.appendHtml(highlighted)
        label1.textChanged.connect(partial(self.rehighlight, label1))
        label1.setTabStopDistance(
            QtGui.QFontMetricsF(label1.font()).horizontalAdvance(' ') * 8)
        queryLayout.addWidget(label1)
        queryWidget.setLayout(queryLayout)
        self.tabwidget.addTab(queryWidget, newQuery["queryname"])

class Connections(QScrollArea):
    def __init__(self, state: dict):
        super().__init__()
        connections_widget = QWidget()
        self.connections_vbox = QVBoxLayout()
        self.connections_vbox.setContentsMargins(0, 1, 0, 0)
        for connection in state.keys():
            domain = QWidget()
            domain_layout = QHBoxLayout()
            domain_layout.setContentsMargins(0, 1, 0, 0)
            object = QLabel(f"{connection[0:20]}...")
            options = QPushButton("use")
            options.setMaximumWidth(60)
            options.clicked.connect(lambda c, x=connection: self.parent().parent()._changeDomain(x))
            domain_layout.addWidget(object, alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
            domain_layout.addWidget(options)
            domain.setLayout(domain_layout)
            self.connections_vbox.addWidget(domain)
        connections_widget.setLayout(self.connections_vbox)
        
        self.setVerticalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOn)
        self.setHorizontalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setWidgetResizable(True)
        self.setWidget(connections_widget)
        self.setMaximumHeight(200)
        self.setMaximumWidth(250)
    
    def addConnection(self, domain_name: str):
        domain = QWidget()
        domain_layout = QHBoxLayout()
        domain_layout.setContentsMargins(0, 1, 0, 0)
        object = QLabel(f"{domain_name[0:20]}...")
        options = QPushButton("use")
        options.setMaximumWidth(60)
        options.clicked.connect(lambda c: self.parent().parent()._changeDomain(domain_name))
        domain_layout.addWidget(object, alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
        domain_layout.addWidget(options)
        domain.setLayout(domain_layout)
        self.connections_vbox.addWidget(domain)

class QueryOptions(QScrollArea):
    def __init__(self, state: dict, domain: str, queries: Queries):
        super().__init__()
        self._domain = domain
        self.queries = queries
        connections_widget = QWidget()
        self.connections_vbox = QVBoxLayout()
        self.connections_vbox.setContentsMargins(0, 1, 0, 0)
        if domain != "":
            for i, query in enumerate(state[domain]["queries"]):
                query1 = QWidget()
                query_layout = QHBoxLayout()
                query_layout.setContentsMargins(0, 1, 0, 0)
                object = QLabel(query["queryname"])
                options = QPushButton("open")
                options.clicked.connect(lambda c, x=query: queries.addQuery(x))
                options.setMaximumWidth(70)
                query_layout.addWidget(object, alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
                query_layout.addWidget(options)
                query1.setLayout(query_layout)
                self.connections_vbox.addWidget(query1)
        connections_widget.setLayout(self.connections_vbox)
        
        self.setVerticalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOn)
        self.setHorizontalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setWidgetResizable(True)
        self.setWidget(connections_widget)
        self.setMaximumHeight(500)
        self.setMaximumWidth(250)
    
    def addQuery(self, queryname: str, _domain: str):
        print("Adding "+ queryname)
        domain = QWidget()
        domain_layout = QHBoxLayout()
        domain_layout.setContentsMargins(0, 1, 0, 0)
        object = QLabel(queryname)
        options = QPushButton("open")
        options.clicked.connect(lambda c: self.queries.addQuery({"queryname": queryname, "datasetid": "", "query": "", "domain": _domain}))
        options.setMaximumWidth(70)
        domain_layout.addWidget(object, alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
        domain_layout.addWidget(options)
        domain.setLayout(domain_layout)
        self.connections_vbox.addWidget(domain)
    
    def resetQueries(self, newState, domain):
        for i in range(self.connections_vbox.count()):
            t = self.connections_vbox.takeAt(0)
            w = t.widget()
            w.setParent(None)
        for i, query in enumerate(newState[domain]["queries"]):
            query1 = QWidget()
            query_layout = QHBoxLayout()
            query_layout.setContentsMargins(0, 1, 0, 0)
            object = QLabel(query["queryname"])
            options = QPushButton("open")
            options.clicked.connect(lambda c, x=query: self.queries.addQuery(x))
            options.setMaximumWidth(70)
            query_layout.addWidget(object, alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
            query_layout.addWidget(options)
            query1.setLayout(query_layout)
            self.connections_vbox.addWidget(query1)

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle('SoQL Studio')
        self.setGeometry(200, 200, 1000, 800)
        self.file = "state.json" # NSBundle.mainBundle().pathForResource_ofType_("state", ".json")
        self.creds_file = "creds.json" # NSBundle.mainBundle().pathForResource_ofType_("creds", ".json")
        try:
            with open(self.file) as f:
                self.state = json.load(f)
        except FileNotFoundError:
            with open(self.file, "w") as f:
                f.write("{}")
                self.state = {}
        self._centralWidget = QWidget(self)
        self._domain = ""
        try:
            self._domain = list(self.state.keys())[0]
            self.queries = Queries(self.state[self._domain]["queries"], self._domain)
        except IndexError:
            self.queries = Queries()
        self.setCentralWidget(self._centralWidget)
        self.connections = Connections(self.state)
        self.query_options = QueryOptions(self.state, self._domain, self.queries)
        self._initLayout()
        self._initCreds()
    
    def _initCreds(self):
        try:
            with open(self.creds_file) as f:
                self.creds = json.load(f)
                user = self.creds["username"]
                pw = keyring.get_password("system", user)
                self.username.setText(user)
                self.password.setText(pw)
        except Exception:
            with open(self.creds_file, "w") as f:
                f.write("{}")

    def _changeDomain(self, domain):
        self._domain = domain
        self.query_options.resetQueries(self.state, self._domain)
        self.queries.connection = domain

    def _writeCreds(self):
        keyring.set_password("system", self.username.text(), self.password.text())
        with open(self.creds_file, "w") as f:
            data = {"username": self.username.text()}
            f.write(json.dumps(data))

    def _initLayout(self):
        layout = QGridLayout()
        left_bar = QVBoxLayout()
        
        creds_layout = QVBoxLayout()
        username_box = QHBoxLayout()
        self.username = QLineEdit()
        self.username.setMaximumWidth(100)
        user_label = QLabel("Username: ")
        user_label.setMaximumWidth(70)
        username_box.addWidget(user_label)
        username_box.addWidget(self.username)
        creds_layout.addLayout(username_box)

        password_box = QHBoxLayout()
        self.password = QLineEdit()
        self.password.setMaximumWidth(100)
        self.password.setEchoMode(QLineEdit.Password)
        pass_label = QLabel("Password: ")
        pass_label.setMaximumWidth(70)
        password_box.addWidget(pass_label)
        password_box.addWidget(self.password)
        creds_layout.addLayout(password_box)

        saveCredsButton = QPushButton("Save Credentials")
        saveCredsButton.clicked.connect(self._writeCreds)
        creds_layout.addWidget(saveCredsButton)

        left_bar.addLayout(creds_layout)


        connection_layout = QVBoxLayout()
        connection_layout.addWidget(QLabel('Connections'), alignment=QtCore.Qt.AlignmentFlag.AlignTop)
        connection_layout.addWidget(self.connections, alignment=QtCore.Qt.AlignmentFlag.AlignTop)
        newConnection = QPushButton("New Connection")
        newConnection.setMaximumWidth(250)
        newConnection.clicked.connect(self._setupConnection)
        connection_layout.addWidget(newConnection, alignment=QtCore.Qt.AlignmentFlag.AlignBottom)
        left_bar.addLayout(connection_layout)

        dataset_layout = QVBoxLayout()
        dataset_layout.addWidget(QLabel('Queries'))
        dataset_layout.addWidget(self.query_options, alignment=QtCore.Qt.AlignmentFlag.AlignTop)
        newQuery = QPushButton("New Query")
        newQuery.setMaximumWidth(250)
        newQuery.clicked.connect(self._setupQuery)
        dataset_layout.addWidget(newQuery, alignment=QtCore.Qt.AlignmentFlag.AlignBottom)
        left_bar.addLayout(dataset_layout)

        layout.addLayout(left_bar, 0, 0, 1, 1)

        layout.addWidget(self.queries, 0, 1, 1, 5)
        self.queries.submitButton.clicked.connect(self._execute)
        self.queries.saveButton.clicked.connect(self._saveQuery)

        self.results_layout = QTableWidget()
        self.results_label = QLabel('Results')
        layout.addWidget(self.results_label, 1, 1, 1, 5)
        layout.addWidget(self.results_layout, 2, 1, 1, 5)
        self._centralWidget.setLayout(layout)
    
    def _setupConnection(self):
        newConnectDialog = ConnectionDialog()
        newConnectDialog.exec_()
        newDomain = newConnectDialog.domain.text()
        if newDomain != "" and newDomain not in self.state.keys():
            self.state[newDomain] = {"queries": []}
            self._saveState()
            self.connections.addConnection(newDomain)
            if self._domain == "":
                self._domain = newDomain

    def _setupQuery(self):
        newQueryDialog = QueryDialog()
        newQueryDialog.exec_()
        newQueryName = newQueryDialog.queryname.text()
        print(newQueryName)
        if newQueryName != "":
            data = {"query": "", "queryname": newQueryName, "datasetid": "", "domain": self._domain}
            self.state[self._domain]["queries"].append(data)
            self.query_options.addQuery(newQueryName, self._domain)
            self._saveState()
            self.queries.addQuery(data)

    def _execute(self):
        tab = self.queries.tabwidget.currentIndex()
        queryWidget = self.queries.tabwidget.widget(tab)
        query: QPlainTextEdit = queryWidget.findChild(QPlainTextEdit)
        datasetidWidget: QLineEdit = queryWidget.findChild(QLineEdit)
        datasetid = datasetidWidget.text()
        domainWidget: QLabel = queryWidget.findChild(QLabel)
        domain = domainWidget.text()
        print(f"executing from tab {tab} for {domain} - {datasetid}: {query.toPlainText()}")
        url = f"https://{domain}/resource/{datasetid}.json?$query={requests.utils.quote(query.toPlainText())}" #
        print(url)
        r = requests.get(url, auth=(self.username.text(),self.password.text()), verify=False)
        self._setResults(r.json())
    
    def _setResults(self, data):
        self.results_layout.reset()
        try:
            self.results_label.setText(f"Results: {len(data)} records")
            if len(data) == 0:
                return
            if len(data) > 10:
                data = data[0:11]
            cols = data[0].keys()
            self.results_layout.setColumnCount(len(cols))
            self.results_layout.setRowCount(len(data))
            self.results_layout.setHorizontalHeaderLabels(cols)
            for i, result in enumerate(data):
                for j, v in enumerate(result.values()):
                    row = QTableWidgetItem(str(v))
                    self.results_layout.setItem(i, j, row)
        except:
            message = data['message'].replace('\n', ' ')[0:50]
            self.results_label.setText(f"Result ERROR: {message}...")
            self.results_layout.reset()

    def _saveQuery(self):
        tab = self.queries.tabwidget.currentIndex()
        queryWidget = self.queries.tabwidget.widget(tab)
        queryName = self.queries.tabwidget.tabText(tab)
        query: QPlainTextEdit = queryWidget.findChild(QPlainTextEdit)
        datasetid: QLineEdit = queryWidget.findChild(QLineEdit)
        queryText = query.toPlainText()
        for i, q in enumerate(self.state[self._domain]["queries"]):
            if q['queryname'] == queryName:
                self.state[self._domain]["queries"][i]["query"] = queryText
                self.state[self._domain]["queries"][i]["datasetid"] = datasetid.text()
        self._saveState()

    def _saveState(self):
        with open(self.file, "w") as f:
            f.write(json.dumps(self.state, indent=4))

if __name__ == "__main__":
    try:
        print("Starting App...")
        app = QApplication([])
        window = MainWindow()
        window.show()
        app.exec_()
    except SystemExit as e:
        print(f"ENDED EARLY: {e}")
