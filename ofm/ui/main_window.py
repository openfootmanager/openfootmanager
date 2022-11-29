import random
import os
import sys

from PySide6 import QtWidgets, QtGui, QtCore


class MainWindow(QtWidgets.QWidget):
    def __init__(self):
        QtWidgets.QWidget.__init__(self)

        self.hello = [
            "Hello World",
            "Olá Mundo",
        ]

        self.button = QtWidgets.QPushButton("Click me")
        self.message = QtWidgets.QLabel("Hellooo!", alignment=QtCore.Qt.AlignCenter)

        self.layout = QtWidgets.QVBoxLayout(self)
        self.layout.addWidget(self.message)
        self.layout.addWidget(self.button)

        self.button.clicked.connect(self.magic)

    @QtCore.Slot()
    def magic(self):
        self.message.text = random.choice(self.hello)


if __name__ == "__main__":
    os.environ['QT_API'] = 'pyside6'
    app = QtWidgets.QApplication(sys.argv)
    widget = MainWindow()
    widget.resize(800, 600)
    widget.show()

    sys.exit(app.exec_())
