'''
Dialog box used to select features which should be ignored.

@author: Donovan Parks
'''

from PyQt4 import QtGui, QtCore
from multCompCorrectionInfoUI import Ui_multCompCorrectionInfoDlg

class MultCompCorrectionInfoDlg(QtGui.QDialog):
  def __init__(self, parent=None, info=None):
    QtGui.QWidget.__init__(self, parent)
    
    # initialize GUI
    self.ui = Ui_multCompCorrectionInfoDlg()
    self.ui.setupUi(self)
        
    # add info to dialog
    pos = 0
    for item in info:
      label = str(item[0]) + ':'
      data = str(item[1])
      
      self.lblLabel = QtGui.QLabel(self)
      self.lblLabel.setText(label)        
      self.ui.layout.setWidget(pos, QtGui.QFormLayout.LabelRole, self.lblLabel)
      
      self.txtData = QtGui.QLineEdit(self)
      self.txtData.setReadOnly(True)
      self.txtData.setText(data)
      self.ui.layout.setWidget(pos, QtGui.QFormLayout.FieldRole, self.txtData)
      
      pos += 1
      
    # add message if there is no additional information
    if len(info) == 0:
      self.lblLabel = QtGui.QLabel(self)
      self.lblLabel.setText("No additional information.")        
      self.ui.layout.setWidget(pos, QtGui.QFormLayout.LabelRole, self.lblLabel)
      pos = 1
       
    # add ok button to dialog
    self.buttonBox = QtGui.QDialogButtonBox(self)
    self.buttonBox.setStandardButtons(QtGui.QDialogButtonBox.Ok)
    self.ui.layout.setWidget(pos, QtGui.QFormLayout.FieldRole, self.buttonBox)
    QtCore.QObject.connect(self.buttonBox, QtCore.SIGNAL("accepted()"), self.accept)
    
    self.adjustSize()

    self.centerWindow()
    
  def centerWindow(self):
    screen = QtGui.QDesktopWidget().screenGeometry()
    size =  self.geometry()
    self.move((screen.width()-size.width())/2, (screen.height()-size.height())/2)
    
if __name__ == "__main__": 
  pass