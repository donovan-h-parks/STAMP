'''
Dialog box used to select features which should be ignored.

@author: Donovan Parks
'''

from PyQt5 import QtGui, QtCore, QtWidgets
from stamp.GUI.multCompCorrectionInfoUI import Ui_multCompCorrectionInfoDlg

class MultCompCorrectionInfoDlg(QtWidgets.QDialog):
  def __init__(self, parent=None, info=None):
    super(MultCompCorrectionInfoDlg, self).__init__(parent)

    # initialize GUI
    self.ui = Ui_multCompCorrectionInfoDlg()
    self.ui.setupUi(self)

    # add info to dialog
    pos = 0
    for item in info:
      label = str(item[0]) + ':'
      data = str(item[1])

      self.lblLabel = QtWidgets.QLabel(self)
      self.lblLabel.setText(label)
      self.ui.layout.setWidget(pos, QtWidgets.QFormLayout.LabelRole, self.lblLabel)

      self.txtData = QtWidgets.QLineEdit(self)
      self.txtData.setReadOnly(True)
      self.txtData.setText(data)
      self.ui.layout.setWidget(pos, QtWidgets.QFormLayout.FieldRole, self.txtData)

      pos += 1

    # add message if there is no additional information
    if len(info) == 0:
      self.lblLabel = QtWidgets.QLabel(self)
      self.lblLabel.setText("No additional information.")
      self.ui.layout.setWidget(pos, QtWidgets.QFormLayout.LabelRole, self.lblLabel)
      pos = 1

    # add ok button to dialog
    self.buttonBox = QtWidgets.QDialogButtonBox(self)
    self.buttonBox.setStandardButtons(QtWidgets.QDialogButtonBox.Ok)
    self.ui.layout.setWidget(pos, QtWidgets.QFormLayout.FieldRole, self.buttonBox)

    # New signal syntax
    self.buttonBox.accepted.connect(self.accept)

    self.adjustSize()

    self.centerWindow()

  def centerWindow(self):
    screen = QtWidgets.QDesktopWidget().screenGeometry()
    size =  self.geometry()
    # Use integer division
    self.move((screen.width()-size.width())//2, (screen.height()-size.height())//2)
    
if __name__ == "__main__": 
  pass